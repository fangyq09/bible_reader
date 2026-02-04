import sqlite3
import re

def process_all_in_one(src_nasb, src_meta, dst_db):
    src_conn = sqlite3.connect(src_nasb)
    meta_conn = sqlite3.connect(src_meta)
    dst_conn = sqlite3.connect(dst_db)
    
    src_cur = src_conn.cursor()
    meta_cur = meta_conn.cursor()
    dst_cur = dst_conn.cursor()

    print("--- 🚀 开始全自动流水线处理 ---")

    # 1. 初始化结构
    dst_cur.executescript("""
        DROP TABLE IF EXISTS verses;
        DROP TABLE IF EXISTS stories;
        DROP TABLE IF EXISTS chapters;
        DROP TABLE IF EXISTS books;
        CREATE TABLE verses (book_num INTEGER, chapter_num INTEGER, verse_num INTEGER, unformatted TEXT, PRIMARY KEY (book_num, chapter_num, verse_num));
        CREATE TABLE stories (book_num INTEGER, chapter_num INTEGER, verse_num INTEGER, order_if_several INTEGER, title TEXT, PRIMARY KEY (book_num, chapter_num, verse_num, order_if_several));
        CREATE TABLE chapters (book_num INTEGER, chapter_num INTEGER, content TEXT, PRIMARY KEY (book_num, chapter_num));
        CREATE TABLE books (number INTEGER PRIMARY KEY, osis TEXT, human TEXT, chapters INTEGER);
    """)

    # 2. 映射书卷 ID
    # 这确保了无论源库如何编号，结果永远是 1=Gen, 66=Rev
    standard_osis = [
        "Gen", "Ex", "Lev", "Num", "Deut", "Josh", "Judg", "Ruth", "1Sam", "2Sam",
        "1Kgs", "2Kgs", "1Chr", "2Chr", "Ezra", "Neh", "Esth", "Job", "Ps", "Prov",
        "Eccl", "Song", "Isa", "Jer", "Lam", "Ezek", "Dan", "Hos", "Joel", "Am",
        "Ob", "Jon", "Mic", "Nah", "Hab", "Zeph", "Hag", "Zech", "Mal", "Mt",
        "Mk", "Lk", "Jn", "Acts", "Rom", "1Cor", "2Cor", "Gal", "Eph", "Phil",
        "Col", "1Ths", "2Ths", "1Tim", "2Tim", "Titus", "Phlm", "Heb", "Jas", "1Pet",
        "2Pet", "1Jn", "2Jn", "3Jn", "Jude", "Rev"
    ]
    osis_to_std_id = {osis: i for i, osis in enumerate(standard_osis, 1)}

    # PH4 的 books 表通常有字段: book_number, short_name
    print("正在通过 OSIS 缩写匹配书卷 ID...")
    src_cur.execute("SELECT book_number, short_name FROM books")
    
    id_map = {}
    for old_id, short_name in src_cur.fetchall():
        # 清理缩写中的空格或特殊字符，匹配标准列表
        clean_short = short_name.strip()
        if clean_short in osis_to_std_id:
            id_map[old_id] = osis_to_std_id[clean_short]
    
    if len(id_map) < 66:
        print(f"⚠️ 警告: 仅匹配到 {len(id_map)} 卷书。次经或非标准缩写已被忽略。")

    # 3. 迁移元数据 (books)
    meta_cur.execute("SELECT osis_code, book_name, total_chapters FROM books_en ORDER BY book_num")
    books_data = [(idx, osis, name, chaps) for idx, (osis, name, chaps) in enumerate(meta_cur.fetchall(), 1)]
    dst_cur.executemany("INSERT INTO books VALUES (?, ?, ?, ?)", books_data)

    # 4. 迁移经文与小标题 (暂不清洗，保留原始标签)
    print("正在迁移原始数据...")
    src_cur.execute("SELECT book_number, chapter, verse, text FROM verses")
    verses_raw = [(id_map[bn], ch, v, txt) for bn, ch, v, txt in src_cur.fetchall() if bn in id_map]
    dst_cur.executemany("INSERT INTO verses VALUES (?, ?, ?, ?)", verses_raw)

    src_cur.execute("SELECT book_number, chapter, verse, order_if_several, title FROM stories")
    stories_raw = [(id_map[bn], ch, v, ord, t) for bn, ch, v, ord, t in src_cur.fetchall() if bn in id_map]
    dst_cur.executemany("INSERT INTO stories VALUES (?, ?, ?, ?, ?)", stories_raw)

    # 5. 聚合章节 (暂不清洗，只做拼接)
    print("正在聚合原始章节内容...")
    story_dict = {}
    for bn, ch, v, _, title in stories_raw:
        story_dict.setdefault((bn, ch, v), []).append(title)

    current_key = None
    chapter_buffer = []
    chapters_to_insert = []

    for bn, ch, v, text in verses_raw:
        key = (bn, ch)
        
        # 构造原始小标题
        prefix = ""
        has_story = False
        if (bn, ch, v) in story_dict:
            has_story = True
            for t in story_dict[(bn, ch, v)]:
                prefix += f"\n\n【{t}】\n\n"

        # 处理经文内的 <pb/> 逻辑（如果是有小标题，则去掉该节开头的 <pb/> 以免空行过多）
        # 此时不做 full clean，只做特定标签的预处理
        verse_text = text
        if has_story and verse_text.startswith("<pb/>"):
            verse_text = verse_text.replace("<pb/>", "", 1)
        
        # 拼接节号
        if verse_text.startswith("<pb/>"):
            formatted_verse = verse_text.replace("<pb/>", f"<pb/>{v} ", 1)
        else:
            formatted_verse = f"{prefix}{v} {verse_text}"

        if key != current_key:
            if current_key is not None:
                chapters_to_insert.append((" ".join(chapter_buffer), current_key[0], current_key[1]))
            current_key = key
            chapter_buffer = [formatted_verse]
        else:
            # 如果这一节本身就是以 <pb/> 开头，拼接时不需要前面的空格
            sep = "" if formatted_verse.startswith("<pb/>") else " "
            chapter_buffer.append(sep + formatted_verse)

    if current_key:
        chapters_to_insert.append((" ".join(chapter_buffer), current_key[0], current_key[1]))
    
    dst_cur.executemany("UPDATE chapters SET content = ? WHERE book_num = ? AND chapter_num = ?", []) # 占位
    # 修正聚合插入逻辑
    dst_cur.executemany("INSERT INTO chapters (content, book_num, chapter_num) VALUES (?, ?, ?)", chapters_to_insert)

    # 6. 执行“正确逻辑”进行最终清洗
    print("正在执行最终清洗逻辑...")
    
    def strip_tags(text, keep_pb_as_newline=False):
        if not text: return ""
        text = re.sub(r'<f>.*?</f>', '', text)
        if keep_pb_as_newline:
            text = text.replace("<pb/>", "\n\n")
        else:
            text = text.replace("<pb/>", "")
        text = re.sub(r'<[^>]+>', '', text)
        return re.sub(r' +', ' ', text).strip()

    # 清洗 chapters
    dst_cur.execute("SELECT book_num, chapter_num, content FROM chapters")
    clean_chapters = [(strip_tags(c, True), bn, ch) for bn, ch, c in dst_cur.fetchall()]
    dst_cur.executemany("UPDATE chapters SET content = ? WHERE book_num = ? AND chapter_num = ?", clean_chapters)

    # 清洗 verses
    dst_cur.execute("SELECT book_num, chapter_num, verse_num, unformatted FROM verses")
    clean_verses = [(strip_tags(t, False), bn, ch, v) for bn, ch, v, t in dst_cur.fetchall()]
    dst_cur.executemany("UPDATE verses SET unformatted = ? WHERE book_num = ? AND chapter_num = ? AND verse_num = ?", clean_verses)

    # 清洗 stories (小标题本身也可能有标签)
    dst_cur.execute("SELECT book_num, chapter_num, verse_num, order_if_several, title FROM stories")
    clean_stories = [(strip_tags(t, False), bn, ch, v, ord) for bn, ch, v, ord, t in dst_cur.fetchall()]
    dst_cur.executemany("UPDATE stories SET title = ? WHERE book_num = ? AND chapter_num = ? AND verse_num = ? AND order_if_several = ?", clean_stories)

    # 7. 提交与优化
    print("正在提交并执行 VACUUM...")
    dst_conn.commit()
    dst_conn.execute("VACUUM")

    src_conn.close()
    meta_conn.close()
    dst_conn.close()
    print("--- ✅ 处理完毕！逻辑完全对齐 ---")

if __name__ == "__main__":
    process_all_in_one("NET.SQLite3", "bible_metadata.db", "net.sqlite3")
