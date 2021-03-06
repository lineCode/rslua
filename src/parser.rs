use crate::{debuggable, error};

use crate::ast::*;
use crate::token::{Token, TokenType, TokenValue};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    debug: bool,
}

#[derive(Debug)]
pub struct SyntaxError(String);

type ParseResult<T> = Result<T, SyntaxError>;

macro_rules! syntax_error {
    ($self:ident, $msg:expr) => {{
        let token = &$self.tokens[$self.current];
        let error_msg;
        match token.value {
            TokenValue::None => {
                error_msg = format!(
                    "[syntax error] {} at line [{}:{}] near [{:?}]",
                    $msg, token.source.line, token.source.col, token.t
                )
            }
            _ => {
                error_msg = format!(
                    "[syntax error] {} at line [{}:{}] near [{:?}]",
                    $msg, token.source.line, token.source.col, token.value
                )
            }
        }
        error!($self, SyntaxError, error_msg)
    }};
}

macro_rules! error_expected {
    ($self:ident, $expected:expr) => {
        syntax_error!($self, &format!("{:?} expected", $expected))?
    };
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            tokens: Vec::new(),
            current: 0,
            debug: false,
        }
    }

    pub fn run(&mut self, tokens: Vec<Token>) -> ParseResult<Block> {
        self.reset();
        self.tokens = tokens;
        self.block()
    }

    // block -> { stat [';'] }
    fn block(&mut self) -> ParseResult<Block> {
        let mut stats: Vec<Stat> = Vec::new();
        while !self.is_block_end() {
            match self.current_token_type() {
                TokenType::Return => {
                    stats.push(self.stat()?);
                    break;
                }
                _ => stats.push(self.stat()?),
            }
        }
        Ok(Block { stats })
    }

    fn stat(&mut self) -> ParseResult<Stat> {
        let line = self.current_line();
        match self.current_token_type() {
            // stat -> ';' (empty stat)
            TokenType::Semi => {
                self.next();
                Ok(Stat::Empty)
            }
            // stat -> if stat
            TokenType::If => Ok(Stat::IfStat(self.ifstat()?)),
            // stat -> while stat
            TokenType::While => Ok(Stat::WhileStat(self.whilestat()?)),
            // stat -> DO block END
            TokenType::Do => {
                self.next();
                let block = self.block()?;
                self.check_match(TokenType::End, TokenType::Do, line)?;
                Ok(Stat::DoBlock(DoBlock { block }))
            }
            // stat -> forstat
            TokenType::For => Ok(Stat::ForStat(self.forstat()?)),
            // stat -> repeatstat
            TokenType::Repeat => Ok(Stat::RepeatStat(self.repeatstat()?)),
            // stat -> funcstat
            TokenType::Function => Ok(Stat::FuncStat(self.funcstat()?)),
            // stat -> localstat
            TokenType::Local => {
                self.next();
                if self.test(TokenType::Function) {
                    Ok(Stat::FuncStat(self.localfunc()?))
                } else {
                    Ok(Stat::LocalStat(self.localstat()?))
                }
            }
            // stat -> label
            TokenType::DbColon => {
                self.next();
                Ok(Stat::LabelStat(self.labelstat()?))
            }
            // stat -> retstat
            TokenType::Return => {
                self.next();
                Ok(Stat::RetStat(self.retstat()?))
            }
            // stat -> breakstat
            TokenType::Break => Ok(Stat::BreakStat(self.breakstat()?)),
            // stat -> gotostat
            TokenType::Goto => Ok(Stat::GotoStat(self.gotostat()?)),
            // stat -> func | assignment
            _ => Ok(self.exprstat()?),
        }
    }

    // ifstat -> IF cond THEN block {ELSEIF cond THEN block} [ELSE block] END
    fn ifstat(&mut self) -> ParseResult<IfStat> {
        let line = self.current_line();
        let mut cond_blocks: Vec<CondBlock> = Vec::new();
        cond_blocks.push(self.test_then_block()?);
        while self.current_token_type() == TokenType::ElseIf {
            cond_blocks.push(self.test_then_block()?);
        }
        let mut else_block = Block::empty();
        if self.test_next(TokenType::Else) {
            else_block = self.block()?;
        }
        self.check_match(TokenType::End, TokenType::If, line)?;
        Ok(IfStat {
            cond_blocks,
            else_block,
        })
    }

    //  [IF | ELSEIF] cond THEN block
    fn test_then_block(&mut self) -> ParseResult<CondBlock> {
        self.next();
        let cond = self.cond()?;
        self.check_next(TokenType::Then)?;
        let block = self.block()?;
        Ok(CondBlock { cond, block })
    }

    // whilestat -> WHILE cond DO block END
    fn whilestat(&mut self) -> ParseResult<WhileStat> {
        let line = self.current_line();
        self.next();
        let cond = self.cond()?;
        self.check_next(TokenType::Do)?;
        let block = self.block()?;
        self.check_match(TokenType::End, TokenType::While, line)?;
        Ok(WhileStat { cond, block })
    }

    fn cond(&mut self) -> ParseResult<Expr> {
        self.expr()
    }

    // forstat -> FOR (fornum | forlist) END
    fn forstat(&mut self) -> ParseResult<ForStat> {
        let line = self.current_line();
        self.next();
        let var_name = self.check_name()?;
        let forstat = match self.current_token_type() {
            TokenType::Assign => self.forenum(&var_name),
            TokenType::Comma | TokenType::In => self.forlist(&var_name),
            _ => syntax_error!(self, "'=' or 'in' expected"),
        };
        match forstat {
            Ok(stat) => {
                self.check_match(TokenType::End, TokenType::For, line)?;
                Ok(stat)
            }
            Err(err) => Err(err),
        }
    }

    // fornum -> NAME = exp1,exp1[,exp1] forbody
    fn forenum(&mut self, var_name: &str) -> ParseResult<ForStat> {
        self.next();
        let init = self.expr()?;
        self.check_next(TokenType::Comma)?;
        let limit = self.expr()?;
        let mut step = None;
        if self.test_next(TokenType::Comma) {
            step = Some(self.expr()?);
        }
        self.check_next(TokenType::Do)?;
        let body = self.block()?;
        Ok(ForStat::ForNum(ForNum {
            var: String::from(var_name),
            init,
            limit,
            step,
            body,
        }))
    }

    // forlist -> NAME {,NAME} IN explist forbody
    fn forlist(&mut self, var_name: &str) -> ParseResult<ForStat> {
        let mut vars: Vec<String> = Vec::new();
        vars.push(String::from(var_name));
        while self.test_next(TokenType::Comma) {
            vars.push(self.check_name()?);
        }
        self.check_next(TokenType::In)?;
        let exprs = self.exprlist()?;
        self.check_next(TokenType::Do)?;
        let body = self.block()?;
        Ok(ForStat::ForList(ForList { vars, exprs, body }))
    }

    // repeatstat -> REPEAT block UNTIL cond
    fn repeatstat(&mut self) -> ParseResult<RepeatStat> {
        let line = self.current_line();
        self.next();
        let block = self.block()?;
        self.check_match(TokenType::Until, TokenType::Repeat, line)?;
        let cond = self.cond()?;
        Ok(RepeatStat { block, cond })
    }

    // funcstat -> FUNCTION funcname body
    fn funcstat(&mut self) -> ParseResult<FuncStat> {
        self.next();
        let func_name = self.funcname()?;
        let body = self.funcbody()?;
        Ok(FuncStat {
            func_type: FuncType::Global,
            func_name,
            body,
        })
    }

    // funcname -> NAME {'.' NAME} [':' NAME]
    fn funcname(&mut self) -> ParseResult<FuncName> {
        let mut fields: Vec<String> = Vec::new();
        fields.push(self.check_name()?);
        while self.test_next(TokenType::Attr) {
            fields.push(self.check_name()?);
        }
        let mut method = None;
        if self.test_next(TokenType::Colon) {
            method = Some(self.check_name()?);
        }
        Ok(FuncName { fields, method })
    }

    // body ->  '(' parlist ')' block END
    fn funcbody(&mut self) -> ParseResult<FuncBody> {
        let line = self.current_line();
        self.check_next(TokenType::Lp)?;
        let mut params: Vec<Param> = Vec::new();
        loop {
            if self.test(TokenType::Rp) {
                break;
            }
            match self.current_token_type() {
                TokenType::Dots => {
                    params.push(Param::VarArg);
                    self.next()
                }
                TokenType::Name => params.push(Param::Name(self.check_name()?)),
                _ => syntax_error!(self, "<name> or '...' expected")?,
            };
            if !self.test_next(TokenType::Comma) {
                break;
            }
        }
        self.check_next(TokenType::Rp)?;
        let block = self.block()?;
        self.check_match(TokenType::End, TokenType::Function, line)?;
        Ok(FuncBody { params, block })
    }

    // funcstat -> local FUNCTION funcname body
    fn localfunc(&mut self) -> ParseResult<FuncStat> {
        self.next();
        let func_name = self.funcname()?;
        let body = self.funcbody()?;
        Ok(FuncStat {
            func_type: FuncType::Local,
            func_name,
            body,
        })
    }

    // stat -> LOCAL NAME {',' NAME} ['=' explist]
    fn localstat(&mut self) -> ParseResult<LocalStat> {
        let mut names: Vec<String> = Vec::new();
        loop {
            names.push(self.check_name()?);
            if !self.test_next(TokenType::Comma) {
                break;
            }
        }
        let mut exprs: Vec<Expr> = Vec::new();
        if self.test_next(TokenType::Assign) {
            exprs = self.exprlist()?;
        }
        Ok(LocalStat { names, exprs })
    }

    // label -> '::' NAME '::'
    fn labelstat(&mut self) -> ParseResult<LabelStat> {
        let label = self.check_name()?;
        self.check_next(TokenType::DbColon)?;
        Ok(LabelStat { label })
    }

    // stat -> RETURN [explist] [';']
    fn retstat(&mut self) -> ParseResult<RetStat> {
        let mut exprs: Vec<Expr> = Vec::new();
        if !self.is_block_end() && self.current_token_type() != TokenType::Semi {
            exprs = self.exprlist()?;
        }
        self.test_next(TokenType::Semi);
        Ok(RetStat { exprs })
    }

    fn breakstat(&mut self) -> ParseResult<BreakStat> {
        self.next();
        Ok(BreakStat {})
    }

    fn gotostat(&mut self) -> ParseResult<GotoStat> {
        self.next();
        let label = self.check_name()?;
        Ok(GotoStat { label })
    }

    // stat -> func call | assignment
    fn exprstat(&mut self) -> ParseResult<Stat> {
        let expr = self.suffixedexpr()?;
        if self.test(TokenType::Assign) || self.test(TokenType::Comma) {
            Ok(Stat::AssignStat(self.assignment(expr)?))
        } else {
            Ok(Stat::CallStat(CallStat { call: expr }))
        }
    }

    // assignment -> ',' suffixedexp assignment
    // assignment -> '=' explist
    fn assignment(&mut self, expr: SuffixedExpr) -> ParseResult<AssignStat> {
        let mut left: Vec<SuffixedExpr> = Vec::new();
        left.push(expr);
        while self.test_next(TokenType::Comma) {
            left.push(self.suffixedexpr()?)
        }
        self.check_next(TokenType::Assign)?;
        let right = self.exprlist()?;
        Ok(AssignStat { left, right })
    }

    // exprlist -> expr { ',' expr }
    fn exprlist(&mut self) -> ParseResult<Vec<Expr>> {
        let mut exprs: Vec<Expr> = Vec::new();
        exprs.push(self.expr()?);
        while self.test_next(TokenType::Comma) {
            exprs.push(self.expr()?)
        }
        Ok(exprs)
    }

    fn expr(&mut self) -> ParseResult<Expr> {
        self.subexpr(0)
    }

    fn get_unop(&self) -> UnOp {
        UnOp::from_token(self.current_token_type())
    }

    fn get_binop(&self) -> BinOp {
        BinOp::from_token(self.current_token_type())
    }

    // subexpr -> (simpleexpr | unop subexpr) { binop subexpr }
    // where 'binop' is any binary operator with a priority higher than 'limit'
    fn subexpr(&mut self, limit: u8) -> ParseResult<Expr> {
        let mut left;
        let unop = self.get_unop();
        if unop != UnOp::None {
            self.next();
            let expr = Box::new(self.subexpr(unop.priority())?);
            left = Expr::UnExpr(UnExpr { op: unop, expr });
        } else {
            left = self.simpleexpr()?;
        }
        let mut binop = self.get_binop();
        while binop != BinOp::None && binop.priority().left > limit {
            self.next();
            let right = self.subexpr(binop.priority().right)?;
            left = Expr::BinExpr(BinExpr {
                left: Box::new(left),
                right: Box::new(right),
                op: binop,
            });
            binop = self.get_binop();
        }
        Ok(left)
    }

    // simpleexpr -> FLT | INT | STRING | NIL | TRUE | FALSE | ... | constructor | FUNCTION body | suffixedexp
    fn simpleexpr(&mut self) -> ParseResult<Expr> {
        let token = self.current_token();
        let expr = match token.t {
            TokenType::Flt => Expr::Float(token.get_float()),
            TokenType::Int => Expr::Int(token.get_int()),
            TokenType::String => Expr::String(token.get_string()),
            TokenType::Nil => Expr::Nil,
            TokenType::True => Expr::True,
            TokenType::False => Expr::False,
            TokenType::Dots => Expr::VarArg,
            TokenType::Lb => return Ok(Expr::Table(self.table()?)),
            TokenType::Function => {
                self.next();
                return Ok(Expr::FuncBody(self.funcbody()?));
            }
            _ => return Ok(Expr::SuffixedExpr(self.suffixedexpr()?)),
        };
        self.next();
        Ok(expr)
    }

    // suffixedexpr -> primaryexpr { '.' NAME | '[' exp ']' | ':' NAME funcargs | funcargs }
    fn suffixedexpr(&mut self) -> ParseResult<SuffixedExpr> {
        let primary = self.primaryexpr()?;
        let mut suffixes: Vec<Suffix> = Vec::new();
        loop {
            match self.current_token_type() {
                TokenType::Attr => {
                    self.next();
                    suffixes.push(Suffix::Attr(self.check_name()?));
                }
                TokenType::Ls => {
                    let line = self.current_line();
                    self.next();
                    suffixes.push(Suffix::Index(self.expr()?));
                    self.check_match(TokenType::Rs, TokenType::Ls, line)?;
                }
                TokenType::Colon => {
                    self.next();
                    let name = self.check_name()?;
                    suffixes.push(Suffix::Method(name));
                }
                TokenType::Lp | TokenType::Lb | TokenType::String => {
                    suffixes.push(Suffix::FuncArgs(self.funcargs()?));
                }
                _ => break,
            }
        }

        Ok(SuffixedExpr { primary, suffixes })
    }

    // primaryexp -> NAME | '(' expr ')'
    fn primaryexpr(&mut self) -> ParseResult<PrimaryExpr> {
        let expr = match self.current_token_type() {
            TokenType::Name => PrimaryExpr::Name(self.check_name()?),
            TokenType::Lp => {
                let line = self.current_line();
                self.next();
                let expr = self.expr()?;
                self.check_match(TokenType::Rp, TokenType::Lp, line)?;
                PrimaryExpr::ParenExpr(Box::new(expr))
            }
            _ => {
                return syntax_error!(
                    self,
                    &format!("unexpected symbol '{:?}'", self.current_token_type())
                )
            }
        };
        Ok(expr)
    }

    // table constructor -> '{' [ field { sep field } [sep] ] '}'
    // sep -> ',' | ';'
    fn table(&mut self) -> ParseResult<Table> {
        let line = self.current_line();
        self.check_next(TokenType::Lb)?;
        let mut fields: Vec<Field> = Vec::new();
        loop {
            if self.test(TokenType::Rb) {
                break;
            }
            fields.push(self.field()?);
            if !self.test_next(TokenType::Comma) && !self.test_next(TokenType::Semi) {
                break;
            }
        }
        self.check_match(TokenType::Rb, TokenType::Lb, line)?;
        Ok(Table { fields })
    }

    // field -> listfield | recfield
    fn field(&mut self) -> ParseResult<Field> {
        let field = match self.current_token_type() {
            TokenType::Name => {
                if self.next_token_type() == TokenType::Assign {
                    self.recfield()?
                } else {
                    self.listfield()?
                }
            }
            TokenType::Ls => self.recfield()?,
            _ => self.listfield()?,
        };
        Ok(field)
    }

    // recfield -> (NAME | '['exp1']') = exp1
    fn recfield(&mut self) -> ParseResult<Field> {
        let key;
        match self.current_token_type() {
            TokenType::Name => key = FieldKey::Name(self.check_name()?),
            TokenType::Ls => {
                let line = self.current_line();
                self.next();
                key = FieldKey::Expr(self.expr()?);
                self.check_match(TokenType::Rs, TokenType::Ls, line)?;
            }
            _ => unreachable!(),
        };
        self.check_next(TokenType::Assign)?;
        let value = self.expr()?;
        Ok(Field::RecFileld(RecField { key, value }))
    }

    // listfield -> expr
    fn listfield(&mut self) -> ParseResult<Field> {
        Ok(Field::ListField(self.expr()?))
    }

    // funcargs -> '(' [ explist ] ') | table constructor | STRING
    fn funcargs(&mut self) -> ParseResult<FuncArgs> {
        let func_args = match self.current_token_type() {
            TokenType::Lp => {
                let line = self.current_line();
                self.next();

                // empty arg list
                if self.test_next(TokenType::Rp) {
                    return Ok(FuncArgs::Exprs(Vec::<Expr>::new()));
                }

                let exprs = self.exprlist()?;
                self.check_match(TokenType::Rp, TokenType::Lp, line)?;
                FuncArgs::Exprs(exprs)
            }
            TokenType::Lb => FuncArgs::Table(self.table()?),
            TokenType::String => {
                let arg = FuncArgs::String(self.current_token().get_string());
                self.next();
                arg
            }
            _ => return syntax_error!(self, "function arguments expected"),
        };
        Ok(func_args)
    }

    fn reset(&mut self) {
        self.current = 0;
    }

    fn current_token(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn next_token(&self) -> &Token {
        &self.tokens[self.current + 1]
    }

    fn current_token_type(&self) -> TokenType {
        let token = self.current_token();
        token.t
    }

    fn current_line(&self) -> usize {
        let token = self.current_token();
        token.source.line
    }

    fn next_token_type(&self) -> TokenType {
        let token = self.next_token();
        token.t
    }

    fn next(&mut self) {
        self.current += 1;
    }

    // if reach a block end
    fn is_block_end(&self) -> bool {
        let token = self.current_token();
        match token.t {
            TokenType::Else
            | TokenType::ElseIf
            | TokenType::End
            | TokenType::Until
            | TokenType::Eos => true,
            _ => false,
        }
    }

    fn check_match(&mut self, end: TokenType, start: TokenType, line: usize) -> ParseResult<()> {
        if self.current_token_type() != end {
            if line == self.current_line() {
                error_expected!(self, end);
            } else {
                syntax_error!(
                    self,
                    &format!("{:?} expected (to close {:?} at line {})", end, start, line)
                )?;
            }
        }
        self.next();
        Ok(())
    }

    fn test(&self, expected: TokenType) -> bool {
        self.current_token_type() == expected
    }

    fn test_next(&mut self, expected: TokenType) -> bool {
        if self.test(expected) {
            self.next();
            true
        } else {
            false
        }
    }

    fn check(&self, expected: TokenType) -> ParseResult<()> {
        if self.current_token_type() != expected {
            error_expected!(self, expected)
        } else {
            Ok(())
        }
    }

    fn check_next(&mut self, expected: TokenType) -> ParseResult<()> {
        self.check(expected)?;
        self.next();
        Ok(())
    }

    fn check_name(&mut self) -> ParseResult<String> {
        self.check(TokenType::Name)?;
        let token = self.current_token();
        let name = match &token.value {
            TokenValue::Str(name) => name.clone(),
            _ => unreachable!(),
        };
        self.next();
        Ok(name)
    }

    debuggable!();
}
