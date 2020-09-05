use std::io::{self, Read};
use std::fs::File;
use std::collections::HashMap;

use lazy_static::lazy_static;

use super::table::KindT;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum KeyId {                   // キーや文字の種類（名前）
    Begin, End,                    // 予約語の名前
    If, Then,
    While, Do,
    Ret, Func,
    Var, Const, Odd,
    Write, WriteLn,
    EndOfKeyWords,                 // 予約語の名前はここまで
    Plus, Minus,                   // 演算子と区切り記号の名前
    Mult, Div,
    Lparen, Rparen,
    Equal, Lss, Gtr,
    NotEq, LssEq, GtrEq,
    Comma, Period, Semicolon,
    Assign,
    EndOfKeySymbol,                // 演算子と区切り記号の名前はここまで
    Id, Num, Nul,                  // トークンの種類
    EndOfToken,
    Letter, Digit, Colon, Other,   // 上記以外の文字の種類
}

impl KeyId {
    pub fn is_key_word(&self) -> bool {                       // キーは予約語か？
        (*self as i32) < (KeyId::EndOfKeyWords as i32)
    }
    pub fn is_key_symbol(&self) -> bool {                     // キーは記号か？
        ((*self as i32) > (KeyId::EndOfKeyWords as i32))
            && ((*self as i32) < KeyId::EndOfKeySymbol as i32)
    }
}

lazy_static! {
    pub static ref KEY_WORD_TABLE: HashMap<&'static str, KeyId> = {  // 予約語や記号と名前（KeyId）の表
        let m: HashMap<&'static str, KeyId> = [
            ("begin",   KeyId::Begin),
            ("end",     KeyId::End),
            ("if",      KeyId::If),
            ("then",    KeyId::Then),
            ("while",   KeyId::While),
            ("do",      KeyId::Do),
            ("return",  KeyId::Ret),
            ("function",KeyId::Func),
            ("var",     KeyId::Var),
            ("const",   KeyId::Const),
            ("odd",     KeyId::Odd),
            ("write",   KeyId::Write),
            ("writeln", KeyId::WriteLn),
            ("$dummy1", KeyId::EndOfKeyWords),        // 記号と名前（KeyId）の表
            ("+",       KeyId::Plus),
            ("-",       KeyId::Minus),
            ("*",       KeyId::Mult),
            ("/",       KeyId::Div),
            ("(",       KeyId::Lparen),
            (")",       KeyId::Rparen),
            ("=",       KeyId::Equal),
            ("<",       KeyId::Lss),
            (">",       KeyId::Gtr),
            ("<>",      KeyId::NotEq),
            ("<=",      KeyId::LssEq),
            (">=",      KeyId::GtrEq),
            (",",       KeyId::Comma),
            (".",       KeyId::Period),
            (";",       KeyId::Semicolon),
            (":=",      KeyId::Assign),
            ("$dummy2", KeyId::EndOfKeySymbol),
        ].iter().cloned().collect();
        m
    };
    pub static ref CHAR_CLASS_TABLE: HashMap<char, KeyId> = {   // 文字の種類を示す表（initCharClassTに相当）
        let mut table = HashMap::new();
        for c in '0'..='9' {
            table.insert(c, KeyId::Digit);
        }
        for c in 'A'..='Z' {
            table.insert(c, KeyId::Letter);
        }
        for c in 'a'..='z' {
            table.insert(c, KeyId::Letter);
        }
        table.insert('+', KeyId::Plus);
        table.insert('-', KeyId::Minus);
        table.insert('*', KeyId::Mult);
        table.insert('/', KeyId::Div);
        table.insert('(', KeyId::Lparen);
        table.insert(')', KeyId::Rparen);
        table.insert('=', KeyId::Equal);
        table.insert('<', KeyId::Lss);
        table.insert('>', KeyId::Gtr);
        table.insert(',', KeyId::Comma);
        table.insert('.', KeyId::Period);
        table.insert(';', KeyId::Semicolon);
        table.insert(':', KeyId::Colon);
        table
    };
}

// const MAXLINE: usize = 120;        // 1行の最大文字数
const MAX_ERROR: i32 = 30;         // これ以上のエラーがあったら終わり
const TAB: i32 = 5;                // タブのスペース
const MAXNAME: usize = 32;         // 名前の最大長さ
const MAXNUM: usize = 14;          // 定数の最大行数

#[derive(Clone, Debug)]
pub enum TokenContent {         // Tokenのunionに相当する型
    Id(String),                 // Identifierの時、その名前
    Value(i32),                 // Numの時、その値
    Nothing,                    // 未初期化時
}

#[derive(Clone, Debug)]
pub struct Token {              // トークンの型
    pub kind: KeyId,            // トークンの種類かキーの名前
    pub u: TokenContent,        // unionに対応する型
}

impl Token {
    pub fn is_st_begin_key(&self) -> bool {     // トークンは文の先頭のキーか？
        match self.kind {
            KeyId::If | KeyId::Begin | KeyId::Ret | KeyId::While | KeyId::Write | KeyId::WriteLn => {
                true
            },
            _ => false
        }
    }
}

pub struct Lexer<'a> {
    lines: std::str::Lines<'a>,           // 次の行を先頭から出力するイテレータ
    line_chars: std::str::Chars<'a>,      // 現在の行の文字を先頭から出力するイテレータ
    line_index: i32,             // 次に読む文字の位置
    ch: char,                    // 最後に読んだ文字
    c_token: Token,              // 最後に読んだトークン
    id_kind: KindT,              // 現トークンの（Id）種類
    spaces: i32,                 // そのトークンの前のスペースの数
    cr: i32,                     // その前のCRの数
    printed: i32,                // トークンは印字済みか
    error_no: i32,               // 出力したエラーの数
}

impl<'a> Lexer<'a> {
    pub fn new(program: &'a String) -> Lexer<'a> { // initSourceに相当。変数の初期設定
        let lines = program.lines();
        let line_chars = "".chars();
        let ch = '\n';
        let c_token = Token { kind: KeyId::Nul, u: TokenContent::Nothing };

        let token_a = Lexer {
            lines, line_chars, line_index: -1, ch,
            c_token, id_kind: KindT::VarId, spaces: 0, cr: 0, printed: 1,  // id_kindの初期値は適当（使用しない）
            error_no: 0,
        };
        token_a
    }
    pub fn error(&mut self, message: &str) {       // 通常のエラーメッセージの出力
        if self.line_index > 0 {
            println!("{:>count$}", "***^", count=(self.line_index as usize));
        } else {
            println!("^");
        }
        println!("*** error *** {}", message);
        self.error_no += 1;
        if self.error_no > MAX_ERROR {             // errorNoCheckの処理に相当
            eprintln!("too many errors");
            println!("abort compilation");
            std::process::exit(1);
        }
    }
    pub fn error_n(&self) -> i32 {                 // エラーの個数を返す
        self.error_no
    }
    fn next_char(&mut self) -> char {              // 次の１文字を返す関数
        if let Some(ch) = self.line_chars.next() {
            self.line_index += 1;
            ch
        } else {
            if let Some(line) = self.lines.next() {
                println!("{}", line);
                self.line_chars = line.chars();
                self.line_index = -1;
                '\n'
            } else {
                self.error("end of file");         // end of fileならコンパイル終了
                std::process::exit(1);
            }
        }
    }
    pub fn next_token(&mut self) -> Token {        // 次のトークンを読んで返す
        self.spaces = 0;
        self.cr = 0;
        loop {
            match self.ch {
                ' ' => self.spaces += 1,
                '\t' => self.spaces += TAB,
                '\n' => {
                    self.spaces = 0;
                    self.cr += 1;
                },
                _    => break
            };
            self.ch = self.next_char();
        }

        let mut temp = Token { kind: KeyId::Nul, u: TokenContent::Nothing };
        let mut ident = String::new();
        let mut i = 0;

        if let Some(cc) = CHAR_CLASS_TABLE.get(&self.ch) {
            match cc {
                KeyId::Letter => {                  // identifier
                    while {
                        if i < MAXNAME {
                            ident.push(self.ch);
                        }
                        i += 1;
                        self.ch = self.next_char();
                        let next_cc = CHAR_CLASS_TABLE.get(&self.ch);
                        if let Some(next_cc) = next_cc {
                            *next_cc == KeyId::Letter || *next_cc == KeyId::Digit
                        } else {
                            false
                        }
                    } {}
                    if i >= MAXNAME {
                        self.error("too long");
                    }
                    if let Some(kind) = KEY_WORD_TABLE.get(&ident.as_str()) {  // 予約語の場合
                        temp.kind = *kind;
                        self.c_token = temp.clone();
                        self.printed = 0;
                        return temp;
                    }
                    temp.kind = KeyId::Id;
                    temp.u = TokenContent::Id(ident);
                },
                KeyId::Digit  => {                  // number
                    let mut num = 0;
                    while {
                        num = 10*num + (self.ch.to_digit(10).unwrap() as i32);
                        i += 1;
                        self.ch = self.next_char();
                        let next_cc = CHAR_CLASS_TABLE.get(&self.ch);
                        if let Some(next_cc) = next_cc {
                            *next_cc == KeyId::Digit
                        } else {
                            false
                        }
                    } {}
                    if i > MAXNUM {
                        self.error("too large");
                    }
                    temp.kind = KeyId::Num;
                    temp.u = TokenContent::Value(num);
                },
                KeyId::Colon  => {
                    self.ch = self.next_char();
                    if self.ch == '=' {
                        self.ch = self.next_char();
                        temp.kind = KeyId::Assign;  // ":="
                    } else {
                        temp.kind = KeyId::Nul;
                    }
                },
                KeyId::Lss    => {
                    self.ch = self.next_char();
                    if self.ch == '=' {
                        self.ch = self.next_char();
                        temp.kind = KeyId::LssEq;   // "<="
                    } else if self.ch == '>' {
                        self.ch = self.next_char();
                        temp.kind = KeyId::NotEq;   // "<>"
                    } else {
                        temp.kind = KeyId::Lss;
                    }
                },
                KeyId::Gtr    => {
                    self.ch = self.next_char();
                    if self.ch == '=' {
                        self.ch = self.next_char();
                        temp.kind = KeyId::GtrEq;   // ">="
                    } else {
                        temp.kind = KeyId::Gtr;
                    }
                },
                _             => {
                    temp.kind = *cc;
                    temp.u = TokenContent::Nothing;
                    if temp.kind != KeyId::Period {
                        self.ch = self.next_char();
                    }
                }
            }
            self.c_token = temp.clone();
            self.printed = 0;
            temp
        } else {
            let temp = Token { kind: KeyId::Other, u: TokenContent::Nothing };
            self.ch = self.next_char();
            self.c_token = temp.clone();
            self.printed = 0;
            temp
        }
    }
    pub fn check_get(&mut self, t: Token, k: KeyId) -> Token {
        /*
        t.kind == k なら、次のトークンを読んで返す
        t.kind != k ならエラーメッセージを出し、tとkが共に記号、または予約語なら
        tを捨て、次のトークンを読んで返す（tをkで置き換えたことになる）
        それ以外の場合、kを挿入したことにして、tを返す
         */
        if t.kind == k {
            return self.next_token();
        }
        if (k.is_key_word() && t.kind.is_key_word()) || (k.is_key_symbol() && t.kind.is_key_symbol()) {
            self.error(format!("delete {:?}, and insert {:?}", self.c_token, k).as_str());
            self.printed = 1;
            return self.next_token();
        }
        self.error(format!("insert {:?}", k).as_str());
        t
    }
    pub fn set_id_kind(&mut self, k: KindT) {     // 現トークン（Id）の種類をセット
        self.id_kind = k;
    }
}

pub fn get_content(filename: String) -> io::Result<String> {
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

pub fn open_source() -> io::Result<String> {       // ソースファイルのopen
    let filename = std::env::args().nth(1);
    if let Some(filename) = filename {
        Ok(get_content(filename)?)
    } else {
        let mut filename = String::new();
        println!("enter source file name");
        io::stdin().read_line(&mut filename)?;
        filename = filename.trim_end().to_string();
        Ok(get_content(filename)?)
    }
}
