use super::{get_source::*, table::*, codegen::*};

const MIN_ERROR: i32 = 3;     // エラーがこれ以下なら実行
const FIRST_ADDR: i32 = 2;    // 各ブロックの最初の変数のアドレス

pub struct Compiler<'a, 'b, 'c, 'd> {
    token: Token,                      // 次のトークンを入れておく
    lex: &'a mut Lexer<'c>,            // 字句解析のメソッドを使うための参照
    gen: &'b mut CodeGenerator<'d>,    // アセンブリ生成のメソッドを使うための参照
}                                      // テーブルへの参照はgenが保持している

impl<'a, 'b, 'c, 'd> Compiler<'a, 'b, 'c, 'd> {
    pub fn new(lex: &'a mut Lexer<'c>, gen: &'b mut CodeGenerator<'d>) -> Compiler<'a, 'b, 'c, 'd> {
        Compiler {
            token: Token { kind: KeyId::Nul, u: TokenContent::Nothing },  // 適当なトークンで初期化する
            lex, gen
        }
    }
    pub fn compile(&mut self) -> bool {
        println!("start compilation:\n");
        self.token = self.lex.next_token();            // 最初のトークン
        self.gen.table.block_begin(FIRST_ADDR);
                                                       // これ以後の宣言は最初のブロックのもの
        self.block(0);                                 // 0はダミー（主ブロックの関数名はない）
        let i = self.lex.error_n();                    // エラーメッセージの個数
        if i != 0 {
            if i == 1 {
                println!("1 error occur");
            } else {
                println!("{} errors occur", i);
            }
        }
        i < MIN_ERROR                                  // エラーメッセージの個数が少ないかどうかの判定
    }
    fn block(&mut self, p_index: i32) {                       // ブロックのコンパイル（p_indexはこのブロックの関数名のインデックス）
        let back_p = self.gen.gen_code_v(OpCode::Jmp, 0);  // 内部関数を飛び越す命令、あとでバックパッチ

        loop {                                             // 宣言部のコンパイルを繰り返す
            match self.token.kind {
                KeyId::Const => {                          // 定数宣言部のコンパイル
                    self.token = self.lex.next_token();
                    self.const_decl();
                },
                KeyId::Var => {                            // 変数宣言部のコンパイル
                    self.token = self.lex.next_token();
                    self.var_decl();
                },
                KeyId::Func => {                           // 関数宣言部のコンパイル
                    self.token = self.lex.next_token();
                    self.func_decl();
                },
                _ => { break; }                            // それ以外なら宣言部は終わり
            }
        }
        self.gen.back_patch(back_p as usize);                        // 内部関数を飛び越す命令にパッチ
        self.gen.table.change_v(p_index, self.gen.next_code());      // この関数の開始番地を修正
        self.gen.gen_code_v(OpCode::Ict, self.gen.table.frame_l());  // このブロックの実行時の必要記憶域をとる命令

        self.statement();                  // このブロックの主文
        self.gen.gen_code_r();             // リターン命令
        self.gen.table.block_end();        // ブロックが終わったことをtableに連絡
    }
    fn const_decl(&mut self) {                                // 定数宣言のコンパイル
        loop {
            if self.token.kind == KeyId::Id {
                self.lex.set_id_kind(KindT::ConstId);                       // 印字のための情報のセット
                let temp = self.token.clone();                              // 名前を入れておく
                let next_token = self.lex.next_token().clone();
                self.token = self.lex.check_get(next_token, KeyId::Equal);  // 次の名前は "=" のはず

                if self.token.kind == KeyId::Num {
                    let id = match temp.u {
                        TokenContent::Id(s) => s,
                        _ => unreachable!(),
                    };
                    let value = match self.token.u {
                        TokenContent::Value(v) => v,
                        _ => unreachable!(),
                    };
                    self.gen.table.enter_table_const(id, value);      // 定数名と値をテーブルに
                } else {
                    self.lex.error("number");
                }
                self.token = self.lex.next_token();
            } else {
                self.lex.error("missing Identifier");
            }
            if self.token.kind != KeyId::Comma {                 // 次がコンマなら定数宣言が続く
                if self.token.kind == KeyId::Id {                // 次が名前ならコンマを忘れたことにする
                    self.lex.error(format!("insert {:?}", KeyId::Comma).as_str());
                    continue;
                } else {
                    break;
                }
            }
            self.token = self.lex.next_token();
        }
        let token = self.token.clone();
        self.token = self.lex.check_get(token, KeyId::Semicolon);  // 最後は ";" のはず
    }
    fn var_decl(&mut self) {                                  // 変数宣言のコンパイル
        loop {
            if self.token.kind == KeyId::Id {
                self.lex.set_id_kind(KindT::VarId);           // 印字のための情報のセット
                let id = match self.token.u.clone() {
                    TokenContent::Id(s) => s,
                    _ => unreachable!(),
                };
                self.gen.table.enter_table_var(id);           // 変数名をテーブルに、番地はtableが決める
                self.token = self.lex.next_token();
            } else {
                self.lex.error("missing Identifier");
            }
            if self.token.kind != KeyId::Comma {              // 次がコンマなら変数宣言が続く
                if self.token.kind == KeyId::Id {             // 次が名前ならコンマを忘れたことにする
                    self.lex.error(format!("insert {:?}", KeyId::Comma).as_str());
                    continue;
                } else {
                    break;
                }
            }
            self.token = self.lex.next_token();
        }
        let token = self.token.clone();
        self.token = self.lex.check_get(token, KeyId::Semicolon);  // 最後は ";" のはず
    }
    fn func_decl(&mut self) {                                 // 関数宣言のコンパイル
        if self.token.kind == KeyId::Id {
            self.lex.set_id_kind(KindT::FuncId);              // 印字のための情報のセット
            let id = match self.token.u.clone() {
                TokenContent::Id(s) => s,
                _ => unreachable!(),
            };
            let f_index = self.gen.table.enter_table_func(id, self.gen.next_code());  // 関数名をテーブルに登録。その先頭番地は、まず、次のコードの番地next_code()とする
            let next_token = self.lex.next_token().clone();
            self.token = self.lex.check_get(next_token, KeyId::Lparen);
            self.gen.table.block_begin(FIRST_ADDR);           // パラメータ名のレベルは関数のブロックと同じ

            loop {
                if self.token.kind == KeyId::Id {             // パラメータ名がある場合、
                    self.lex.set_id_kind(KindT::ParId);       // 印字のための情報をセット
                    let id = match self.token.u.clone() {
                        TokenContent::Id(s) => s,
                        _ => unreachable!(),
                    };
                    self.gen.table.enter_table_par(id);       // パラメータ名をテーブルに登録
                    self.token = self.lex.next_token();
                } else {
                    break;
                }
                if self.token.kind != KeyId::Comma {          // 次がコンマならパラメータ名が続く
                    if self.token.kind == KeyId::Id {         // 次が名前ならコンマを忘れたことにする
                        self.lex.error(format!("insert {:?}", KeyId::Comma).as_str());
                        continue;
                    } else {
                        break;
                    }
                }
                self.token = self.lex.next_token();
            }
            let token = self.token.clone();
            self.token = self.lex.check_get(token, KeyId::Rparen);  // 最後は ")" のはず
            self.gen.table.end_par();                               // パラメータ部が終わったことをテーブルに連絡
            if self.token.kind == KeyId::Semicolon {
                println!("delete {:?}", KeyId::Semicolon);
                self.token = self.lex.next_token();
            }
            self.block(f_index);                     // ブロックのコンパイル、その関数名のインデックスを渡す
            let token = self.token.clone();
            self.token = self.lex.check_get(token, KeyId::Semicolon);  // 最後は ";" のはず
        } else {
            self.lex.error("missing identifier");
        }
    }
    fn statement(&mut self) {                                 // 文のコンパイル
        loop {
            match self.token.kind {
                KeyId::Id => {                                // 代入文のコンパイル
                    let id = match self.token.u.clone() {
                        TokenContent::Id(s) => s,
                        _ => unreachable!(),
                    };
                    let t_index = self.gen.table.search_t(id, KindT::VarId);  // 左辺の変数のインデックス
                    let k = self.gen.table.kind_t(t_index);                   // 印字のための情報のセット
                    self.lex.set_id_kind(k);
                    if k != KindT::VarId && k != KindT::ParId {       // 変数名かパラメータ名のはず
                        self.lex.error("type error: var/par");
                    }
                    let next_token = self.lex.next_token().clone();
                    self.token = self.lex.check_get(next_token, KeyId::Assign);  // ":=" のはず
                    self.expression();                                // 式のコンパイル
                    self.gen.gen_code_t(OpCode::Sto, t_index);        // 左辺への代入命令
                    return;
                },
                KeyId::If => {                                // if文のコンパイル
                    self.token = self.lex.next_token();
                    self.condition();                         // 条件式のコンパイル
                    let token = self.token.clone();
                    self.token = self.lex.check_get(token, KeyId::Then);  // "then" のはず
                    let back_p = self.gen.gen_code_v(OpCode::Jpc, 0);     // jpc命令
                    self.statement();                         // 文のコンパイル
                    self.gen.back_patch(back_p as usize);     // 上のjpc命令にバックパッチ
                    return;
                },
                KeyId::Ret => {                               // return文のコンパイル
                    self.token = self.lex.next_token();
                    self.expression();                        // 式のコンパイル
                    self.gen.gen_code_r();                    // ret命令
                    return;
                },
                KeyId::Begin => {                             // begin . . end文のコンパイル
                    self.token = self.lex.next_token();
                    loop {
                        self.statement();                     // 文のコンパイル
                        loop {
                            if self.token.kind == KeyId::Semicolon {  // 次が ";" なら文が続く
                                self.token = self.lex.next_token();
                                break;
                            }
                            if self.token.kind == KeyId::End {        // 次がendなら終わり
                                self.token = self.lex.next_token();
                                return;
                            }
                            if self.token.is_st_begin_key() {         // 次が文の先頭記号なら ";" を忘れたことにする
                                self.lex.error(format!("insert {:?}", KeyId::Semicolon).as_str());
                                break;
                            }
                            println!("delete {:?}", self.token.kind);  // それ以外ならエラーとして読み捨てる
                            self.token = self.lex.next_token();
                        }
                    }
                },
                KeyId::While => {                             // while文のコンパイル
                    self.token = self.lex.next_token();
                    let back_p2 = self.gen.next_code();       // while文の最後のjmp命令の飛び先
                    self.condition();                         // 条件式のコンパイル
                    let token = self.token.clone();
                    self.token = self.lex.check_get(token, KeyId::Do);  // "do" のはず
                    let back_p = self.gen.gen_code_v(OpCode::Jpc, 0);   // 条件式が偽のとき飛び出すjpc命令
                    self.statement();                           // 文のコンパイル
                    self.gen.gen_code_v(OpCode::Jmp, back_p2);  // while文の先頭へのjmp命令
                    self.gen.back_patch(back_p as usize);     // 偽のとき飛び出すjpc命令へのバックパッチ
                    return;
                },
                KeyId::Write => {                             // write文のコンパイル
                    self.token = self.lex.next_token();
                    self.expression();                        // 式のコンパイル
                    self.gen.gen_code_o(Operator::Wrt);       // その値を出力するwrt命令
                    return;
                },
                KeyId::WriteLn => {                           // writeln文のコンパイル
                    self.token = self.lex.next_token();
                    self.gen.gen_code_o(Operator::Wrl);       // 改行を出力するwrl命令
                    return;
                },
                KeyId::End | KeyId::Semicolon | KeyId::Period => {  // Follow statement のトークンの場合
                    return;                                         // 空文を読んだことにして終わり
                },
                _ => {                                         // 文の先頭のキーまで読み捨てる
                    println!("delete {:?}", self.token.kind);  // 今読んだトークンを読み捨てる
                    self.token = self.lex.next_token();
                    continue;
                }
            }
        }
    }
    fn expression(&mut self) {                                // 式のコンパイル
        let mut k = self.token.kind.clone();
        if k == KeyId::Plus || k == KeyId::Minus {
            self.token = self.lex.next_token();
            self.term();
            if k == KeyId::Minus {
                self.gen.gen_code_o(Operator::Neg);
            }
        } else {
            self.term();
        }
        k = self.token.kind.clone();
        while k == KeyId::Plus || k == KeyId::Minus {
            self.token = self.lex.next_token();
            self.term();
            if k == KeyId::Minus {
                self.gen.gen_code_o(Operator::Sub);
            } else {
                self.gen.gen_code_o(Operator::Add);
            }
            k = self.token.kind.clone();
        }
    }
    fn term(&mut self) {                                      // 式の項のコンパイル
        self.factor();
        let mut k = self.token.kind.clone();
        while k == KeyId::Mult || k == KeyId::Div {
            self.token = self.lex.next_token();
            self.factor();
            if k == KeyId::Mult {
                self.gen.gen_code_o(Operator::Mul);
            } else {
                self.gen.gen_code_o(Operator::Div);
            }
            k = self.token.kind.clone();
        }
    }
    fn factor(&mut self) {                                    // 式の因子のコンパイル
        if self.token.kind == KeyId::Id {
            let id = match self.token.u.clone() {
                TokenContent::Id(s) => s,
                _ => unreachable!(),
            };
            let t_index = self.gen.table.search_t(id, KindT::VarId);
            let k = self.gen.table.kind_t(t_index);           // 印字のための情報のセット
            self.lex.set_id_kind(k);
            match k {
                KindT::VarId | KindT::ParId => {              // 変数名かパラメータ名
                    self.gen.gen_code_t(OpCode::Lod, t_index);
                    self.token = self.lex.next_token();
                },
                KindT::ConstId => {                           // 定数名
                    self.gen.gen_code_v(OpCode::Lit, self.gen.table.val(t_index));
                    self.token = self.lex.next_token();
                },
                KindT::FuncId => {                            // 関数呼び出し
                    self.token = self.lex.next_token();
                    if self.token.kind == KeyId::Lparen {
                        let mut i = 0;                        // iは実引数の個数
                        self.token = self.lex.next_token();
                        if self.token.kind != KeyId::Rparen {
                            loop {
                                self.expression();            // 実引数のコンパイル
                                i += 1;
                                if self.token.kind == KeyId::Comma {  // 次がコンマなら実引数が続く
                                    self.token = self.lex.next_token();
                                    continue;
                                }
                                let token = self.token.clone();
                                self.token = self.lex.check_get(token, KeyId::Rparen);
                                break;
                            }
                        } else {
                            self.token = self.lex.next_token();
                        }
                        if self.gen.table.pars(t_index) != i {  // pars(t_index) は仮引数の個数
                            self.lex.error("unmatched par");
                        }
                    } else {
                        self.lex.error(format!("insert {:?}", KeyId::Lparen).as_str());
                        self.lex.error(format!("insert {:?}", KeyId::Rparen).as_str());
                    }
                    self.gen.gen_code_t(OpCode::Cal, t_index);  // call命令
                }
            }
        } else if self.token.kind == KeyId::Num {             // 定数
            let value = match self.token.u.clone() {
                TokenContent::Value(v) => v,
                _ => unreachable!(),
            };
            self.gen.gen_code_v(OpCode::Lit, value);
            self.token = self.lex.next_token();
        } else if self.token.kind == KeyId::Lparen {          // 「(」「因子」「)」
            self.token = self.lex.next_token();
            self.expression();
            let token = self.token.clone();
            self.token = self.lex.check_get(token, KeyId::Rparen);
        }
        match self.token.kind {                               // 因子のあとがまた因子ならエラー
            KeyId::Id | KeyId::Num | KeyId::Lparen => {
                self.lex.error(format!("missing operator: {:?}", self.token.kind).as_str());
                self.factor();
            },
            _ => (),
        }
    }
    fn condition(&mut self) {                                 // 条件式のコンパイル
        if self.token.kind == KeyId::Odd {
            self.token = self.lex.next_token();
            self.expression();
            self.gen.gen_code_o(Operator::Odd);
        } else {
            self.expression();
            let k = self.token.kind.clone();
            match k {
                KeyId::Equal | KeyId::Lss | KeyId::Gtr | KeyId::NotEq | KeyId::LssEq | KeyId::GtrEq => {},
                _ => {
                    self.lex.error("type error: rel-op");
                }
            }
            self.token = self.lex.next_token();
            self.expression();
            match k {
                KeyId::Equal => self.gen.gen_code_o(Operator::Eq),
                KeyId::Lss => self.gen.gen_code_o(Operator::Ls),
                KeyId::Gtr => self.gen.gen_code_o(Operator::Gr),
                KeyId::NotEq => self.gen.gen_code_o(Operator::Neq),
                KeyId::LssEq => self.gen.gen_code_o(Operator::Lseq),
                KeyId::GtrEq => self.gen.gen_code_o(Operator::Greq),
                _ => unreachable!(),
            };
        }
    }
    pub fn print_code(&self) {
        println!("\ninstructions for the virtual machine:");
        self.gen.print_code();
    }
    pub fn execute(&self) {
        println!("\nstart execution:");
        self.gen.execute();
    }
}
