use std;
use std::cell::RefCell;
use std::io::Error;
use std::rc::Rc;
use api::*;
use stmt;
use typ::*;
use variable::Source;

#[derive(Clone, Debug)]
pub struct Argument {
	pub ty: Type,
	pub expr: stmt::Expression,
}
impl Argument {
	pub fn new(t: &Type, s: Rc<RefCell<Source>>) -> Self {
		use variable;
		use std::ops::Deref;
		let exp = stmt::Expression::Simple(variable::ScalarOp::Null,
		                                   s.borrow().deref().clone());
		Argument{ty: t.clone(), expr: exp}
	}
	pub fn newexpr(t: &Type, expression: &stmt::Expression) -> Self {
		Argument{ty: t.clone(), expr: expression.clone()}
	}
	pub fn source(&self) -> Source {
		match self.expr {
			stmt::Expression::Simple(_, ref s) => s.clone(),
			_ => unimplemented!(),
		}
	}

	pub fn codegen(&self, strm: &mut std::io::Write, pgm: &Program)
		-> Result<(),Error> {
		use stmt::Code;
		self.expr.codegen(strm, pgm)
	}
}

#[derive(Clone, Debug)]
pub struct ReturnType {
	pub ty: Type,
	pub src: Rc<RefCell<Source>>,
}
impl ReturnType {
	pub fn new(t: &Type, s: Rc<RefCell<Source>>) -> Self {
		ReturnType{ty: t.clone(), src: s}
	}
}

#[derive(Clone, Debug)]
pub struct Function {
	pub retval: ReturnType,
	pub arguments: Vec<Argument>,
	pub name: String,
}
impl Function {
	pub fn new(nm: &str, rettype: &ReturnType, args: &Vec<Argument>) -> Self {
		Function{
			name: nm.to_string(),
			retval: rettype.clone(),
			arguments: args.clone(),
		}
	}
}
