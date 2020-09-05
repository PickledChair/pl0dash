use pl0dash::{
    get_source::{open_source, Lexer},
    table::NameTable,
    codegen::CodeGenerator,
    compile::Compiler,
};

fn main() {
    let content = match open_source() {                       // ソースプログラムの内容を得る
        Ok(content) => content,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };
    let mut lex = Lexer::new(&content);                       // 字句解析のための変数を設定
    let mut table_ = NameTable::new();                        // 名前表を作成
    let mut gen = CodeGenerator::new(&mut table_);            // アセンブリ生成のための変数を設定
    let mut compiler = Compiler::new(&mut lex, &mut gen);     // ワンパスコンパイルのための変数を設定
    if compiler.compile() {                                   // コンパイルして、
        if let Some(flag) = std::env::args().nth(2) {         // 成功したとき
            if flag.as_str() == "-p" {                        // -p フラグを渡されているときは
                compiler.print_code();                        // 仮想機械のアセンブリを印字
            } else {                                          // そうでなければ
                compiler.execute();                           // アセンブリを仮想機械上で実行
            }
        } else {
            compiler.execute();                               // フラグ指定がなければ実行
        }
    }
}
