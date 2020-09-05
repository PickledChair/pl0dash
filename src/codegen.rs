use super::table::{RelAddr, NameTable};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OpCode {                                 // 命令語のコード
    Lit, Opr, Lod, Sto, Cal, Ret, Ict, Jmp, Jpc,
}

// impl OpCode {
//     pub fn as_str(&self) -> &'static str {
//         match *self {
//             OpCode::Lit => "lit",
//             OpCode::Opr => "opr",
//             OpCode::Lod => "lod",
//             OpCode::Sto => "sto",
//             OpCode::Cal => "cal",
//             OpCode::Ret => "ret",
//             OpCode::Ict => "ict",
//             OpCode::Jmp => "jmp",
//             OpCode::Jpc => "jpc",
//         }
//     }
// }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Operator {                               // 演算命令のコード
    Neg, Add, Sub, Mul, Div, Odd, Eq, Ls, Gr,
    Neq, Lseq, Greq, Wrt, Wrl,
}

// impl Operator {
//     pub fn as_str(&self) -> &'static str {
//         match *self {
//             Operator::Neg  => "neg",
//             Operator::Add  => "add",
//             Operator::Sub  => "sub",
//             Operator::Mul  => "mul",
//             Operator::Div  => "div",
//             Operator::Odd  => "odd",
//             Operator::Eq   => "eq",
//             Operator::Ls   => "ls",
//             Operator::Gr   => "gr",
//             Operator::Neq  => "neq",
//             Operator::Lseq => "lseq",
//             Operator::Greq => "greq",
//             Operator::Wrt  => "wrt",
//             Operator::Wrl  => "wrl",
//         }
//     }
// }

const MAXCODE: usize = 100;     // 目的コードの最大長さ
const MAXMEM: usize = 2000;     // 実行時スタックの最大長さ
const MAXREG: usize = 20;       // 演算レジスタスタックの最大長さ
const MAXLEVEL: usize = 5;      // ブロックの最大深さ

#[derive(Copy, Clone, Debug)]
pub enum InstU {
    RelAddr(RelAddr),
    Value(i32),
    Operator(Operator),
}

#[derive(Copy, Clone, Debug)]
pub struct Inst {               // 命令語の型
    op_code: OpCode,
    u: InstU,
}

pub struct CodeGenerator<'a> {
    code: Vec<Inst>,                  // 目的コードが入る
    c_index: i32,                     // 最後に生成した命令語のインデックス
    pub table: &'a mut NameTable,
}

impl<'a> CodeGenerator<'a> {
    pub fn new(table: &'a mut NameTable) -> CodeGenerator<'a> {
        CodeGenerator { code: Vec::new(), c_index: -1, table }
    }
    pub fn next_code(&self) -> i32 {                                  // 次の命令語のアドレスを返す
        self.c_index + 1
    }
    fn check_max(&mut self) {                                         // 目的コードのインデックスの増加とチェック
        self.c_index += 1;
        if self.c_index < MAXCODE as i32 {
            return;
        } else {
            println!("too many code");
            std::process::exit(1);
        }
    }
    pub fn gen_code_v(&mut self, op: OpCode, v: i32) -> i32 {         // 命令語の生成、アドレス部にv
        self.check_max();
        self.code.push(Inst { op_code: op, u: InstU::Value(v) });
        self.c_index
    }
    pub fn gen_code_t(&mut self, op: OpCode, ti: i32) -> i32 {        // 命令語の生成、アドレスは名前表から
        self.check_max();
        self.code.push(Inst { op_code: op, u: InstU::RelAddr(self.table.rel_addr(ti)) });
        self.c_index
    }
    pub fn gen_code_o(&mut self, p: Operator) -> i32 {                // 命令語の生成、アドレス部に演算命令
        self.check_max();
        self.code.push(Inst { op_code: OpCode::Opr, u: InstU::Operator(p) });
        self.c_index
    }
    pub fn gen_code_r(&mut self) -> i32 {                             // ret命令語の生成
        if self.code[self.c_index as usize].op_code == OpCode::Ret {  // 直前がretなら生成せず
            return self.c_index;
        }
        self.check_max();
        self.code.push(Inst { op_code: OpCode::Ret, u: InstU::RelAddr(
            RelAddr {
                level: self.table.block_level(),
                addr: self.table.func_pars().unwrap()                 // パラメータ数（実行スタックの解放用）
            }
        )});
        self.c_index
    }
    pub fn back_patch(&mut self, i: usize) {                          // 命令語のバックパッチ（次の番地を）
        self.code[i].u = InstU::Value(self.c_index + 1);
    }
    // pub fn print_code(&self, i: usize) {   // 命令語の印字
    //     let op_code = self.code[i].op_code;
    //     match self.code[i].u {
    //         InstU::Value(v) => {
    //             println!("{},{}", op_code.as_str(), v);
    //         },
    //         InstU::RelAddr(r) => {
    //             println!("{},{},{}", op_code.as_str(), r.level, r.addr);
    //         }
    //         InstU::Operator(o) => {
    //             println!("{},{}", op_code.as_str(), o.as_str());
    //         }
    //     }
    // }
    // pub fn list_code(&self) {         // 目的コード（命令語）のリスティング
    //     use std::io::Write;
    //     println!("\ncode");
    //     for i in 0..self.code.len() {
    //         print!("{: >3}", i);
    //         std::io::stdout().flush().unwrap();
    //         self.print_code(i);
    //     }
    // }
    pub fn print_code(&self) {            // 目的コード（命令語）のリスティング
        for c in self.code.iter() {
            println!("{:?}", c);
        }
    }
    pub fn execute(&self) {               // 目的コード（命令語）の実行
        let mut stack: [i32; MAXMEM] = [0; MAXMEM];         // 実行時スタック
        let mut display: [i32; MAXLEVEL] = [0; MAXLEVEL];   // 現在見える各ブロックの先頭番地のディスプレイ

        let mut pc: usize = 0;             // pc: 命令語のカウンタ
        let mut top: usize = 0;            // top: 次にスタックに入れる場所

        stack[0] = 0; stack[1] = 0;
        // stack[top] は callee で壊すディスプレイの退避場所
        // stack[top+1] は caller への戻り番地
        display[0] = 0;
        // 主ブロックの先頭番地は 0

        while {
            let i = self.code[pc];         // これから実行する命令語
            pc += 1;
            match i.op_code {
                OpCode::Lit => {
                    stack[top] = match i.u {
                        InstU::Value(v) => v,
                        _ => unreachable!(),
                    };
                    top += 1;
                },
                OpCode::Lod => {
                    let index = match i.u {
                        InstU::RelAddr(r) => { (display[r.level as usize] + r.addr) as usize },
                        _ => unreachable!(),
                    };
                    stack[top] = stack[index];
                    top += 1;
                },
                OpCode::Sto => {
                    let index = match i.u {
                        InstU::RelAddr(r) => { (display[r.level as usize] + r.addr) as usize },
                        _ => unreachable!(),
                    };
                    top -= 1;
                    stack[index] = stack[top];
                },
                OpCode::Cal => {
                    // r.level は callee の名前のレベル
                    // callee のブロックのレベル lev はそれに＋１したもの
                    let (level, addr) = match i.u {
                        InstU::RelAddr(r) => (r.level as usize, r.addr as usize),
                        _ => unreachable!(),
                    };
                    let lev = level + 1;
                    stack[top] = display[lev];   // display[lev] の退避
                    stack[top + 1] = pc as i32;
                    display[lev] = top as i32;   // 現在の top が callee のブロックの先頭番地
                    pc = addr;
                },
                OpCode::Ret => {
                    let (level, addr) = match i.u {
                        InstU::RelAddr(r) => (r.level as usize, r.addr as usize),
                        _ => unreachable!(),
                    };
                    top -= 1;
                    let temp = stack[top];            // スタックのトップにあるものが返す値
                    top = display[level] as usize;    // top を呼ばれたときの値に戻す
                    display[level] = stack[top];      // 壊したディスプレイの回復
                    pc = stack[top + 1] as usize;
                    top -= addr;                      // 実引数の分だけトップを戻す
                    stack[top] = temp;                // 返す値をスタックのトップへ
                    top += 1;
                },
                OpCode::Ict => {
                    let v = match i.u {
                        InstU::Value(v) => v as usize,
                        _ => unreachable!(),
                    };
                    top += v;
                    if top >= MAXMEM - MAXREG {
                        println!("stack overflow");
                        std::process::exit(1);
                    }
                },
                OpCode::Jmp => {
                    let v = match i.u {
                        InstU::Value(v) => v as usize,
                        _ => unreachable!(),
                    };
                    pc = v;
                },
                OpCode::Jpc => {
                    top -= 1;
                    if stack[top] == 0 {
                        let v = match i.u {
                            InstU::Value(v) => v as usize,
                            _ => unreachable!(),
                        };
                        pc = v;
                    }
                },
                OpCode::Opr => {
                    let optr = match i.u {
                        InstU::Operator(optr) => optr,
                        _ => unreachable!(),
                    };
                    match optr {
                        Operator::Neg => {
                            stack[top-1] = -stack[top-1];
                        },
                        Operator::Add => {
                            top -= 1;
                            stack[top-1] += stack[top];
                        },
                        Operator::Sub => {
                            top -= 1;
                            stack[top-1] -= stack[top];
                        },
                        Operator::Mul => {
                            top -= 1;
                            stack[top-1] *= stack[top];
                        },
                        Operator::Div => {
                            top -= 1;
                            stack[top-1] /= stack[top];
                        },
                        Operator::Odd => {
                            stack[top-1] = stack[top-1] & 1;
                        },
                        Operator::Eq => {
                            top -= 1;
                            stack[top-1] = (stack[top-1] == stack[top]) as i32;
                        },
                        Operator::Ls => {
                            top -= 1;
                            stack[top-1] = (stack[top-1] < stack[top]) as i32;
                        },
                        Operator::Gr => {
                            top -= 1;
                            stack[top-1] = (stack[top-1] > stack[top]) as i32;
                        },
                        Operator::Neq => {
                            top -= 1;
                            stack[top-1] = (stack[top-1] != stack[top]) as i32;
                        },
                        Operator::Lseq => {
                            top -= 1;
                            stack[top-1] = (stack[top-1] <= stack[top]) as i32;
                        },
                        Operator::Greq => {
                            top -= 1;
                            stack[top-1] = (stack[top-1] >= stack[top]) as i32;
                        },
                        Operator::Wrt => {
                            use std::io::Write;
                            top -= 1;
                            print!("{}", stack[top]);
                            std::io::stdout().flush().unwrap();
                        },
                        Operator::Wrl => {
                            println!("");
                        }
                    }
                },
            }
            pc != 0
        } {}
    }
}
