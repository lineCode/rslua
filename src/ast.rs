use crate::token::TokenType;
use crate::types::{FloatType, IntType};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum UnOp {
    Minus,
    BNot,
    Not,
    TLen,
    None,
}

impl UnOp {
    pub fn from_token(token: TokenType) -> UnOp {
        match token {
            TokenType::Minus => UnOp::Minus,
            TokenType::BXor => UnOp::BNot,
            TokenType::Not => UnOp::Not,
            TokenType::TLen => UnOp::TLen,
            _ => UnOp::None,
        }
    }
    pub fn priority(self) -> u8 {
        match self {
            _ => 12,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum BinOp {
    Add,
    Minus,
    Mul,
    Mod,
    Pow,
    Div,
    IDiv,
    BAnd,
    BOr,
    BXor,
    Shl,
    Shr,
    Concat,
    Ne,
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    None,
}

pub struct BinOpPriority {
    pub left: u8,
    pub right: u8,
}

impl BinOp {
    pub fn from_token(token: TokenType) -> BinOp {
        match token {
            TokenType::Add => BinOp::Add,
            TokenType::Minus => BinOp::Minus,
            TokenType::Mul => BinOp::Mul,
            TokenType::Mod => BinOp::Mod,
            TokenType::Pow => BinOp::Pow,
            TokenType::Div => BinOp::Div,
            TokenType::IDiv => BinOp::IDiv,
            TokenType::BAnd => BinOp::BAnd,
            TokenType::BOr => BinOp::BOr,
            TokenType::BXor => BinOp::BXor,
            TokenType::Shl => BinOp::Shl,
            TokenType::Shr => BinOp::Shr,
            TokenType::Concat => BinOp::Concat,
            TokenType::Ne => BinOp::Ne,
            TokenType::Eq => BinOp::Eq,
            TokenType::Lt => BinOp::Lt,
            TokenType::Le => BinOp::Le,
            TokenType::Gt => BinOp::Gt,
            TokenType::Ge => BinOp::Ge,
            TokenType::And => BinOp::And,
            TokenType::Or => BinOp::Or,
            _ => BinOp::None,
        }
    }
    pub fn priority(self) -> BinOpPriority {
        match self {
            BinOp::Or => BinOpPriority { left: 1, right: 1 },
            BinOp::And => BinOpPriority { left: 2, right: 2 },
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                BinOpPriority { left: 3, right: 3 }
            }
            BinOp::BOr => BinOpPriority { left: 4, right: 4 },
            BinOp::BXor => BinOpPriority { left: 5, right: 5 },
            BinOp::BAnd => BinOpPriority { left: 6, right: 6 },
            BinOp::Shl | BinOp::Shr => BinOpPriority { left: 7, right: 7 },
            BinOp::Concat => BinOpPriority { left: 9, right: 8 },
            BinOp::Add | BinOp::Minus => BinOpPriority {
                left: 10,
                right: 10,
            },
            BinOp::Mul | BinOp::Mod | BinOp::Div | BinOp::IDiv => BinOpPriority {
                left: 11,
                right: 11,
            },
            BinOp::Pow => BinOpPriority {
                left: 14,
                right: 13,
            },
            _ => unreachable!(),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Expr {
    Nil,
    True,
    False,
    Float(FloatType),
    Int(IntType),
    String(String),
    FuncBody(FuncBody),
    Table(Table),
    BinExpr(BinExpr),
    UnExpr(UnExpr),
    VarArg,
    SuffixedExpr(SuffixedExpr),
}

#[derive(PartialEq, Debug)]
pub struct SuffixedExpr {
    pub primary: PrimaryExpr,
    pub suffixes: Vec<Suffix>,
}

#[derive(PartialEq, Debug)]
pub enum PrimaryExpr {
    Name(String),
    ParenExpr(Box<Expr>),
}

#[derive(PartialEq, Debug)]
pub enum Suffix {
    Attr(String),
    Index(Expr),
    Method(String),
    FuncArgs(FuncArgs),
}

#[derive(PartialEq, Debug)]
pub enum FuncArgs {
    Exprs(Vec<Expr>),
    Table(Table),
    String(String),
}

#[derive(PartialEq, Debug)]
pub struct Table {
    pub fields: Vec<Field>,
}

#[derive(PartialEq, Debug)]
pub enum Field {
    ListField(Expr),
    RecFileld(RecField),
}

#[derive(PartialEq, Debug)]
pub struct RecField {
    pub key: FieldKey,
    pub value: Expr,
}

#[derive(PartialEq, Debug)]
pub enum FieldKey {
    Name(String),
    Expr(Expr),
}

#[derive(PartialEq, Debug)]
pub struct UnExpr {
    pub op: UnOp,
    pub expr: Box<Expr>,
}

#[derive(PartialEq, Debug)]
pub struct BinExpr {
    pub op: BinOp,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
}

#[derive(PartialEq, Debug)]
pub struct IfStat {
    pub cond_blocks: Vec<CondBlock>,
    pub else_block: Block,
}

#[derive(PartialEq, Debug)]
pub struct CondBlock {
    pub cond: Expr,
    pub block: Block,
}

#[derive(PartialEq, Debug)]
pub struct WhileStat {
    pub cond: Expr,
    pub block: Block,
}

#[derive(PartialEq, Debug)]
pub struct DoBlock {
    pub block: Block,
}

#[derive(PartialEq, Debug)]
pub enum ForStat {
    ForNum(ForNum),
    ForList(ForList),
}

#[derive(PartialEq, Debug)]
pub struct ForNum {
    pub var: String,
    pub init: Expr,
    pub limit: Expr,
    pub step: Option<Expr>,
    pub body: Block,
}

#[derive(PartialEq, Debug)]
pub struct ForList {
    pub vars: Vec<String>,
    pub exprs: Vec<Expr>,
    pub body: Block,
}

#[derive(PartialEq, Debug)]
pub struct RepeatStat {
    pub cond: Expr,
    pub block: Block,
}

#[derive(PartialEq, Debug)]
pub enum FuncType {
    Global,
    Local,
}

#[derive(PartialEq, Debug)]
pub struct FuncStat {
    pub func_type: FuncType,
    pub func_name: FuncName,
    pub body: FuncBody,
}

#[derive(PartialEq, Debug)]
pub struct FuncName {
    pub fields: Vec<String>,
    pub method: Option<String>,
}

#[derive(PartialEq, Debug)]
pub struct FuncBody {
    pub params: Vec<Param>,
    pub block: Block,
}

#[derive(PartialEq, Debug)]
pub enum Param {
    VarArg,
    Name(String),
}

#[derive(PartialEq, Debug)]
pub struct LocalStat {
    pub names: Vec<String>,
    pub exprs: Vec<Expr>,
}

#[derive(PartialEq, Debug)]
pub struct LabelStat {
    pub label: String,
}

#[derive(PartialEq, Debug)]
pub struct RetStat {
    pub exprs: Vec<Expr>,
}

#[derive(PartialEq, Debug)]
pub struct BreakStat {}

#[derive(PartialEq, Debug)]
pub struct GotoStat {
    pub label: String,
}

#[derive(PartialEq, Debug)]
pub struct AssignStat {
    pub left: Vec<SuffixedExpr>,
    pub right: Vec<Expr>,
}

#[derive(PartialEq, Debug)]
pub struct CallStat {
    pub call: SuffixedExpr,
}

#[derive(PartialEq, Debug)]
pub enum Stat {
    Empty,
    IfStat(IfStat),
    WhileStat(WhileStat),
    DoBlock(DoBlock),
    ForStat(ForStat),
    RepeatStat(RepeatStat),
    FuncStat(FuncStat),
    LocalStat(LocalStat),
    LabelStat(LabelStat),
    RetStat(RetStat),
    BreakStat(BreakStat),
    GotoStat(GotoStat),
    AssignStat(AssignStat),
    CallStat(CallStat),
}

#[derive(PartialEq, Debug)]
pub struct Block {
    pub stats: Vec<Stat>,
}

impl Block {
    pub fn empty() -> Self {
        Block { stats: Vec::new() }
    }
}
