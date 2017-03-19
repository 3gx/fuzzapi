// This provides a type system and API for parsing code.
//
// The type system here mirrors typ::Type, but we use more strings here
// to reference types instead of fully instantiating them.  This makes
// sense for parsing, because it lets us parse without worrying too
// much about semantics, and thereby importantly means we do less error
// handling during parsing and more during subsequent semantic analysis.
use typ::{Decl, EnumValue, Type};
use variable;

#[derive(Debug)]
pub enum DeclType {
	Basic(Type),
	Struct(Vec<UDTDecl>),
	Enum(Vec<EnumValue>),
	StructRef(String),
	EnumRef(String),
}

#[derive(Debug)]
pub struct UDTDecl {
	pub name: String,
	pub ty: DeclType,
}

#[derive(Debug)]
pub struct FreeVarDecl {
	pub name: String,
	pub op: variable::ScalarOp,
	pub genname: String,
	pub ty: DeclType, // Struct(...) and Enum(...) are not valid, but *Refs are.
}

#[derive(Debug)]
pub struct FuncDecl {
	pub name: String,
	pub retval: DeclType,
	pub arguments: Vec<DeclType>,
}

#[derive(Debug, PartialEq)]
pub struct FuncCall {
	pub name: String,
	pub retval: String, // the type name encoded as a string
	pub arguments: Vec<String>, // arguments encoded as a string.
}

#[derive(Debug)]
pub enum Declaration {
	Free(FreeVarDecl),
	Function(FuncDecl),
	UDT(UDTDecl),
}

// gives the type from the declaration.
// it needs to take the current type list as well, because this DeclType may
// reference other types, and it would need to produce boxes to those types.
fn type_from_decl(decl: &DeclType) -> Type {
	match decl {
		&DeclType::Basic(ref ty) => ty.clone(),
		&DeclType::Struct(ref udt) => {
			let mut flds: Vec<(String, Box<Type>)> = Vec::new();
			for f in udt {
				match f.ty {
					DeclType::Basic(ref ty) =>
						flds.push(("_unnamed_".to_string(), Box::new(ty.clone()))),
					DeclType::Struct(ref st) => {
						for s in st {
							let subtype = type_from_decl(&s.ty);
							flds.push(("_unnamed2_".to_string(), Box::new(subtype)));
						}
					},
					DeclType::Enum(ref en) => {
						let v = Type::Enum("_unnamed_enum_".to_string(), en.clone());
						flds.push(("_unnamed3_".to_string(), Box::new(v)));
					}
					DeclType::StructRef(/*ref nm*/ _) => unimplemented!(),
					DeclType::EnumRef(/*ref nm*/ _) => unimplemented!(),
				}
			}
			Type::Struct("_unnamed_struct_".to_string(), flds)
		},
		&DeclType::Enum(_) => unimplemented!(),
		&DeclType::StructRef(_) => unimplemented!(),
		&DeclType::EnumRef(_) => unimplemented!(),
	}
}

// replaces the "Decl" types from this module with the typ::* counterparts,
// potentially panic'ing due to invalid semantics.
fn resolve_types(decls: &Vec<Declaration>) ->
	(Vec<Decl>, Vec<variable::Source>) {
	assert!(decls.len() > 0);
	let mut drv: Vec<Decl> = Vec::new();

	for decl in decls {
		match decl {
			&Declaration::Free(ref fvar) => {
				drv.push(Decl::Ty(type_from_decl(&fvar.ty)));
			},
			&Declaration::Function(ref fqn) => {},
			&Declaration::UDT(ref udecl) => {},
		};
	}
	(drv, vec![])
}

#[cfg(test)]
mod test {
	use api;
	use fuzz;
	use typ::{Native, Type};

	#[test]
	fn test_empty_struct() {
		let s = "struct entry { }";
		assert!(fuzz::parse_L_API(s).is_ok());
		assert_eq!(fuzz::parse_L_API(s).unwrap().len(), 1);
		let ref decl: api::Declaration = fuzz::parse_L_API(s).unwrap()[0];
		let decl = match decl {
			&api::Declaration::UDT(ref udt) => udt,
			_ => panic!("invalid declaration parse {:?}", decl),
		};
		assert_eq!(decl.name, "entry".to_string());
		match decl.ty {
			api::DeclType::Basic(_) => panic!("type should be UDT, is Basic"),
			api::DeclType::Enum(_) => panic!("type should be UDT, is Enum"),
			api::DeclType::EnumRef(_) => panic!("type should be UDT, is EnumRef"),
			api::DeclType::StructRef(_) => panic!("type should be UDT, is StructRef"),
			api::DeclType::Struct(ref decllist) => {
				assert_eq!(decllist.len(), 0)
			},
		};
	}

	#[test]
	fn test_struct_pointer_char() {
		let s = "struct Ent { pointer char key; }";
		assert!(fuzz::parse_L_API(s).is_ok());
		assert_eq!(fuzz::parse_L_API(s).unwrap().len(), 1);
		let ref decl: api::Declaration = fuzz::parse_L_API(s).unwrap()[0];
		let decl = match decl {
			&api::Declaration::UDT(ref udt) => udt,
			_ => panic!("invalid declaration parse {:?}", decl),
		};
		assert_eq!(decl.name, "Ent".to_string());
		match decl.ty {
			api::DeclType::Basic(_) => panic!("type should be UDT, is Basic"),
			api::DeclType::Enum(_) => panic!("type should be UDT, is Enum"),
			api::DeclType::EnumRef(_) => panic!("type should be UDT, is EnumRef"),
			api::DeclType::StructRef(_) => panic!("type should be UDT, is StructRef"),
			api::DeclType::Struct(ref decllist) => {
				assert_eq!(decllist.len(), 1);
				let ref key: api::UDTDecl = decllist[0];
				assert_eq!(key.name, "key");
				match key.ty {
					api::DeclType::Struct(_) => panic!("incorrect type UDT for 'key'"),
					api::DeclType::Enum(_) => panic!("incorrect type Enum for 'key'"),
					api::DeclType::EnumRef(_) => panic!("incorrect type for 'key'"),
					api::DeclType::StructRef(_) => panic!("incorrect type for 'key'"),
					api::DeclType::Basic(ref blt) => {
						let ch = Type::Builtin(Native::Character);
						assert_eq!(blt, &Type::Pointer(Box::new(ch)));
					}
				}
			},
		};
	}

	#[test]
	fn test_struct_multiple_fields() {
		let s = "struct Entry {\n".to_string() +
			"pointer char key;\n" +
			"pointer void value;\n" +
		"}";
		assert!(fuzz::parse_L_API(s.as_str()).is_ok());
		assert_eq!(fuzz::parse_L_API(s.as_str()).unwrap().len(), 1);
		let ref decl: api::Declaration = fuzz::parse_L_API(s.as_str()).unwrap()[0];
		let decl = match decl {
			&api::Declaration::UDT(ref udt) => udt,
			_ => panic!("invalid declaration parse {:?}", decl),
		};
		assert_eq!(decl.name, "Entry".to_string());
		match decl.ty {
			api::DeclType::Basic(_) => panic!("type should be UDT, is Basic"),
			api::DeclType::Enum(_) => panic!("type should be UDT, is Enum"),
			api::DeclType::EnumRef(_) => panic!("type should be UDT, is EnumRef"),
			api::DeclType::StructRef(_) => panic!("type should be UDT, is StructRef"),
			api::DeclType::Struct(ref decllist) => {
				assert_eq!(decllist.len(), 2);
				let ref key: api::UDTDecl = decllist[0];
				assert_eq!(key.name, "key");
				match key.ty {
					api::DeclType::Struct(_) => panic!("incorrect type UDT for 'key'"),
					api::DeclType::Enum(_) => panic!("incorrect type Enum for 'key'"),
					api::DeclType::EnumRef(_) => panic!("incorrect type for 'key'"),
					api::DeclType::StructRef(_) => panic!("incorrect type for 'key'"),
					api::DeclType::Basic(ref blt) => {
						let ch = Type::Builtin(Native::Character);
						assert_eq!(blt, &Type::Pointer(Box::new(ch)));
					}
				}
				let ref value: api::UDTDecl = decllist[1];
				assert_eq!(value.name, "value");
				match value.ty {
					api::DeclType::Struct(_) => panic!("incorrect type UDT for 'key'"),
					api::DeclType::Enum(_) => panic!("incorrect type Enum for 'key'"),
					api::DeclType::EnumRef(_) => panic!("incorrect type for 'key'"),
					api::DeclType::StructRef(_) => panic!("incorrect type for 'key'"),
					api::DeclType::Basic(ref blt) => {
						let ch = Type::Builtin(Native::Void);
						assert_eq!(blt, &Type::Pointer(Box::new(ch)));
					}
				}
			},
		};
	}

	#[test]
	fn test_enum_single() {
		let s = "enum Enumeration { BLAH = 0 , }";
		match fuzz::parse_L_API(s) {
			Ok(_) => {},
			Err(e) => panic!("{:?}", e),
		};
		let t = "enum Enumeration { BLA = 0 , }";
		assert!(fuzz::parse_L_API(t).is_ok());
		assert_eq!(fuzz::parse_L_API(t).unwrap().len(), 1);
	}

	#[test]
	fn test_enum_multi() {
		let s = "enum Enumeration { FOO = 0 , BAR = 1 , BAZ = 42 , }";
		let decls = match fuzz::parse_L_API(s) {
			Ok(parsed) => parsed,
			Err(e) => panic!("{:?}", e),
		};
		assert_eq!(decls.len(), 1);
	}

	#[test]
	fn test_struct_fvar_single() {
		let s = "struct X { } var:free blah op:null gen:I32 i32";
		let decls = match fuzz::parse_L_API(s) {
			Ok(parsed) => parsed,
			Err(e) => panic!("{:?}", e),
		};
		assert_eq!(decls.len(), 2);
	}

	#[test]
	fn test_parse_function_new() {
		let s = "function:new hcreate_r int {usize, pointer struct hsearch_data,}";
		let decls: Vec<api::Declaration> = match fuzz::parse_L_API(s) {
			Ok(parsed) => parsed,
			Err(e) => panic!("{:?}", e),
		};
		assert_eq!(decls.len(), 1);
		let fqn = match decls[0] {
			api::Declaration::Function(ref f) => f,
			_ => panic!("non function type {:?}", decls[0]),
		};
		assert_eq!(fqn.name, "hcreate_r");
		match fqn.retval {
			api::DeclType::Basic(ref ty) => match ty {
				&Type::Builtin(ref t) => assert_eq!(*t, Native::Integer),
				_ => panic!("basic type, but {:?}, not integer", ty),
			},
			_ => panic!("retval should be a basic type, not {:?}", fqn.retval),
		};
		assert_eq!(fqn.arguments.len(), 2);
		match fqn.arguments[0] {
			api::DeclType::Basic(ref ty) => match ty {
				&Type::Builtin(ref t) => assert_eq!(*t, Native::Usize),
				_ => panic!("basic type, but {:?} not usize", ty),
			},
			_ => panic!("arg0 should be a basic type, not {:?}", fqn.arguments[0]),
		};
		let ptr: &Type = match fqn.arguments[1] {
			api::DeclType::Basic(ref ptr) => ptr,
			_ => panic!("invalid arg1: {:?}", fqn.arguments[1]),
		};
		let boxptr = match ptr {
			&Type::Pointer(ref b) => b,
			_ => panic!("invalid ptr type {:?}", ptr),
		};
		use std::ops::Deref;
		match boxptr.deref() {
			&Type::Struct(ref nm, _) => assert_eq!(nm, "hsearch_data"),
			_ => panic!("invalid box ptr {:?}", boxptr),
		};
	}

	#[test]
	fn test_parse_two_function_decls() {
		let s = "function:new hcreate_r int {".to_string() +
			"usize, pointer struct hsearch_data," +
		"}" +
		"function:new hsearch_r int {" +
			"int, int, pointer pointer int, pointer struct hsearch_data," +
		"}";
		let decls: Vec<api::Declaration> = match fuzz::parse_L_API(s.as_str()) {
			Ok(parsed) => parsed,
			Err(e) => panic!("{:?}", e),
		};
		assert_eq!(decls.len(), 2);
		let fqn = match decls[0] {
			api::Declaration::Function(ref f) => f,
			_ => panic!("non function type {:?}", decls[0]),
		};
		assert_eq!(fqn.name, "hcreate_r");
	}
}
