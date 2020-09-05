use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum KindT {                      // Identifierの種類
    VarId, FuncId, ParId, ConstId,
}

// impl KindT {
//     pub fn kind_name(&self) -> &'static str {   // 名前の種類の出力用関数
//         match *self {                           // (Debugトレイトだけでなんとかなるので未実装)
//             KindT::VarId => "var",
//             KindT::FuncId => "func",
//             KindT::ParId => "par",
//             KindT::ConstId => "const",
//         }
//     }
// }

#[derive(Copy, Clone, Debug)]
pub struct RelAddr {                  // 変数、パラメータ、関数のアドレスの型
    pub level: i32,
    pub addr: i32,
}

const MAXTABLE: usize = 100;          // 名前表の最大長さ
// const MAXNAME: usize = 31;            // 名前の最大長さ
const MAXLEVEL: usize = 5;            // ブロックの最大深さ

#[derive(Copy, Clone, Debug)]
pub enum TableEntryU {                // unionに相当する型
    Value(i32),                          // 定数の場合：値
    Func { raddr: RelAddr, pars: i32 },  // 関数の場合：先頭アドレス、パラメータ数
    RelAddr(RelAddr),                    // 変数・パラメータの場合：アドレス
}

#[derive(Clone, Debug)]
pub struct TableEntry {               // 名前表のエントリの型
    kind: KindT,                      // 名前の種類
    name: String,                     // 名前の綴り
    u: TableEntryU,                   // unionに相当する型
}

#[derive(Clone, Debug)]
pub struct NameTable {
    table: HashMap<i32, TableEntry>,  // 名前表
    t_index: i32,                     // 名前表のインデックス
    level: i32,                       // 現在のブロックレベル
    index: [i32; MAXLEVEL],           // index[i]にはブロックレベルiの最後のインデックス
    addr: [i32; MAXLEVEL],            // addr[i]にはブロックレベルiの最後の変数の番地
    local_addr: i32,                  // 現在のブロックの最後の変数の番地
    tf_index: i32,                    // 名前表の関数名のインデックス
}

impl NameTable {
    pub fn new() -> NameTable {
        NameTable {
            table: HashMap::new(),
            t_index: 0,
            level: -1,
            index: [0; MAXLEVEL],
            addr: [0; MAXLEVEL],
            local_addr: 0,
            tf_index: 0,
        }
    }
    pub fn block_begin(&mut self, first_addr: i32) {        // ブロックの始まり（最初の変数の番地）で呼ばれる
        if self.level == -1 {                    // 主ブロックの時、初期設定
            self.local_addr = first_addr;
            self.t_index = 0;
            self.level += 1;
            return;
        }
        if self.level == (MAXLEVEL - 1) as i32 {
            eprintln!("too many nested blocks");
            std::process::exit(1);
        }
        self.index[self.level as usize] = self.t_index;     // 今までのブロックの情報を格納
        self.addr[self.level as usize] = self.local_addr;
        self.local_addr = first_addr;                       // 新しいブロックの最初の変数の番地
        self.level += 1;                                    // 新しいブロックのレベル
    }
    pub fn block_end(&mut self) {                           // ブロックの終わりで呼ばれる
        self.level -= 1;
        if self.level > -1 {
            self.t_index = self.index[self.level as usize];  // 一つ外側のブロックの情報を回復
            self.local_addr = self.addr[self.level as usize];
        }
        // self.t_index = self.index[self.level as usize];
        // self.local_addr = self.addr[self.level as usize];
    }
    pub fn  block_level(&self) -> i32 {                     // 現ブロックのレベルを返す
        self.level
    }
    pub fn func_pars(&self) -> Option<i32> {                // 現ブロックの関数のパラメータ数を返す
        if self.level > 0 {
            self.table.get(&self.index[(self.level - 1) as usize])
                .map_or(None, |entry| {
                    match entry.u {
                        TableEntryU::Func {raddr: _, pars: p} => Some(p),
                        _ => None
                    }
                })
        } else {
            Some(0)
        }
        // self.table.get(&self.index[(self.level - 1) as usize])
        //     .map_or(None, |entry| {
        //         match entry.u {
        //             TableEntryU::Func {raddr: _, pars: p} => Some(p),
        //             _ => None
        //         }
        //     })
    }
    pub fn enter_table_func(&mut self, id: String, v: i32) -> i32 {  // 名前表に関数名と先頭番地を登録
        self.t_index += 1;
        if self.t_index < MAXTABLE as i32 {
            self.table.insert(self.t_index, TableEntry {
                kind: KindT::FuncId,
                name: id,
                u: TableEntryU::Func {
                    raddr: RelAddr { level: self.level, addr: v },   // addr: 関数の先頭番地
                    pars: 0,                                         // pars: パラメータ数の初期値
                }
            });
            self.tf_index = self.t_index;
            self.t_index
        } else {
            eprintln!("too many names");
            std::process::exit(1);
        }
    }
    pub fn enter_table_par(&mut self, id: String) -> i32 {  // 名前表にパラメータ名を登録
        self.t_index += 1;
        if self.t_index < MAXTABLE as i32 {
            self.table.insert(self.t_index, TableEntry {
                kind: KindT::ParId,
                name: id,
                u: TableEntryU::RelAddr(RelAddr { level: self.level, addr: 0 })
            });
            let entry = self.table.get(&self.tf_index).unwrap().clone();
            match entry.u {
                TableEntryU::Func { raddr: r, pars: p } => {
                    self.table.insert(self.tf_index, TableEntry {
                        u: TableEntryU::Func { raddr: r, pars: p + 1 },  // 関数のパラメータ数のカウント
                        ..entry
                    })
                },
                _ => unreachable!(),
            };
            self.t_index
        } else {
            eprintln!("too many names");
            std::process::exit(1);
        }
    }
    pub fn enter_table_var(&mut self, id: String) -> i32 {  // 名前表に変数名を登録
        self.t_index += 1;
        if self.t_index < MAXTABLE as i32 {
            self.table.insert(self.t_index, TableEntry {
                kind: KindT::VarId,
                name: id,
                u: TableEntryU::RelAddr(RelAddr {
                    level: self.level, addr: self.local_addr,
                })
            });
            self.local_addr += 1;
            self.t_index
        } else {
            eprintln!("too many names");
            std::process::exit(1);
        }
    }
    pub fn enter_table_const(&mut self, id: String, v: i32) -> i32 {  // 名前表に定数名とその値を登録
        self.t_index += 1;
        if self.t_index < MAXTABLE as i32 {
            self.table.insert(self.t_index, TableEntry {
                kind: KindT::ConstId,
                name: id,
                u: TableEntryU::Value(v)
            });
            self.t_index
        } else {
            eprintln!("too many names");
            std::process::exit(1);
        }
    }
    pub fn end_par(&mut self) {                             // パラメータ宣言部の最後で呼ばれる
        let pars = match self.table.get(&self.tf_index).unwrap().u {
            TableEntryU::Func { raddr: _, pars: p } => p,
            _ => unreachable!(),
        };
        if pars == 0 { return; }
        for i in 1..=(pars as usize) {                      // 各パラメータの番地を決める
            let entry = self.table.get(&(self.tf_index+(i as i32))).unwrap().clone();
            match entry.u {
                TableEntryU::RelAddr(r) => {
                    self.table.insert(self.tf_index+(i as i32), TableEntry {
                        u: TableEntryU::RelAddr(RelAddr {
                            level: r.level, addr: (i as i32) - 1 - pars,
                        }),
                        ..entry
                    });
                },
                _ => unreachable!(),
            }
        }
    }
    pub fn change_v(&mut self, ti: i32, new_val: i32) {  // 名前表.get(&ti)の値（関数の先頭番地）の変更
        let entry = self.table.get(&ti);
        if let Some(entry) = entry {
            let entry = entry.clone();
            match entry.u {
                TableEntryU::Func { raddr: r, pars: p } => {
                    self.table.insert(ti, TableEntry {
                        u: TableEntryU::Func { raddr: RelAddr {
                            addr: new_val, ..r
                        }, pars: p },
                        ..entry
                    });
                },
                _ => unreachable!(),
            }
        } else {                                  // ti=0の時、つまり主ブロックのエントリを作成していないので、新しいエントリを作成
            self.table.insert(ti, TableEntry {
                kind: KindT::FuncId,
                name: String::from(""),
                u: TableEntryU::Func { raddr: RelAddr {
                    addr: new_val, level: 0,
                }, pars: 0 },
            });
        }
    }
    pub fn search_t(&mut self, id: String, k: KindT) -> i32 {  // 名前idの名前表の位置を返す
        for (index, entry) in self.table.iter() {
            if entry.name == id { return *index; }                 // 名前があった
        }
        // Lexerのerrorメソッドを使うのにミュータブルな参照が必要なので、errorを吐けない
        // error出力のために別のモジュールを設けるべきなのかもしれない
        if k == KindT::VarId { return self.enter_table_var(id); }  // 名前がなかったら、変数の時は仮登録
        0
    }
    pub fn kind_t(&self, i: i32) -> KindT {                 // 名前表.get(&i)の種類を返す
        self.table.get(&i).unwrap().kind
    }
    pub fn rel_addr(&self, ti: i32) -> RelAddr {            // 名前表.get(&ti)のアドレスを返す
        let entry = self.table.get(&ti).unwrap();
        match entry.u {
            TableEntryU::RelAddr(r) => r,
            TableEntryU::Func { raddr: r, pars: _ } => r,   // nameTable[ti].u.raddrで共用体のメンバにアクセスする時、
                                                            // nameTable[ti].f.u.raddrにもアクセスできていると考えた
            _ => unreachable!(),
        }
    }
    pub fn val(&self, ti: i32) -> i32 {                     // 名前表.get(&ti)のTableEntryU::Value(value)のvalueを返す
        let entry = self.table.get(&ti).unwrap();
        match entry.u {
            TableEntryU::Value(v) => v,
            _ => unreachable!(),
        }
    }
    pub fn pars(&self, ti: i32) -> i32 {                    // 名前表.get(&ti)の関数のパラメータ数を返す
        let entry = self.table.get(&ti).unwrap();
        match entry.u {
            TableEntryU::Func { raddr: _, pars: p } => p,
            _ => unreachable!(),
        }
    }
    pub fn frame_l(&self) -> i32 {     // そのブロックで実行時に必要とするメモリ容量
        self.local_addr
    }
}
