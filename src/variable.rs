// This holds information about a variable is used in an API.  Breifly:
//   Source: where the variable comes from / how it is generated
//   Use: where the variable is consumed, i.e. which parameter to which fqn
//   Generator: holds the current/next state in the TypeClass list (tc.rs)
//   Free: container for everything.  code gen.
use function::*;
use typ::*;
use tc::*;

// Identifies the source of a variable.  Variable values can be generated via
// return values, e.g. 'x = f()', or as paramater args, e.g. 'type x; f(&x);'.
#[allow(dead_code)]
pub enum Source {
	Free,
	Parameter(Function, usize), // function + parameter index it comes from
	ReturnValue(Function),
}

// Details where a value is used: which parameter of which function.
pub enum Use {
	Nil, // isn't used.
	Argument(Function, usize), // function + parameter index it comes from
}

// Free is a container for all of the variable information.
pub trait Free {
	fn typename(&self) -> String;
	fn name(&self) -> String;
	// Generate a C expression that could be used in initializing a value of this
	// variable.
	fn value(&self) -> String;
}

// A dependent variable is a variable that we don't actually have control over.
// For example, if the API model states that 'the return value of f() must be
// the second argument of g()', a la:
//   type v = f();
//   g(_, v);
// Then 'v' is dependent.  The effect is mostly that we don't attach a
// Generator to it.
pub struct Dependent {
	pub name: String,
	pub src: Source,
	pub dest: Use,
	pub ty: Type,
}

// A Generator holds TypeClass information and helps us iterate through the
// class of all values by knowing where we are in that sequence.
pub trait Generator {
	// Grabs the current state as an expression.
	fn get(&self) -> String;
	// Moves to the next state.  Does nothing if at the end state.
	fn next(&mut self);
	fn n_state(&self) -> usize;
}

pub fn create(t: &Type) -> Box<Generator> {
	match t {
		&Type::Enum(_, _) => Box::new(GenEnum::create(t)),
		&Type::I32 => Box::new(GenI32::create(t)),
		&Type::Pointer(_) => Box::new(GenPointer::create(t)),
		&Type::Field(_, ref x) => create(x),
		_ => panic!("unimplemented type {:?}", t), // for no valid reason
	}
}

//---------------------------------------------------------------------

pub struct GenEnum {
	cls: TC_Enum,
	idx: usize, // index into the list of values that this enum can take on
}

impl GenEnum {
	pub fn create(t: &Type) -> Self {
		GenEnum{cls: TC_Enum::new(t), idx: 0}
	}
}

impl Generator for GenEnum {
	fn get(&self) -> String {
		return self.cls.value(self.idx).to_string();
	}
	fn next(&mut self) {
		if self.idx < self.cls.n()-1 {
			self.idx = self.idx + 1;
		}
	}

	fn n_state(&self) -> usize {
		return self.cls.n();
	}
}

pub struct GenI32 {
	cls: TC_I32,
	idx: usize,
}

impl GenI32 {
	pub fn create(_: &Type) -> Self {
		GenI32{ cls: TC_I32::new(), idx: 0 }
	}
}

impl Generator for GenI32 {
	fn get(&self) -> String {
		return self.cls.value(self.idx).to_string();
	}
	fn next(&mut self) {
		if self.idx < self.cls.n()-1 {
			self.idx = self.idx + 1
		}
	}

	fn n_state(&self) -> usize {
		return self.cls.n();
	}
}

pub struct GenUDT {
	types: Vec<Type>,
	values: Vec<Box<Generator>>,
	idx: Vec<usize>,
}

impl GenUDT {
	pub fn create(t: &Type) -> Self {
		// UDT's 2nd tuple param is a Vec<Box<Type>>, but we want a Vec<Type>.
		let tys: Vec<Type> = match t {
			&Type::UDT(_, ref types) =>
				types.iter().map(|x| (**x).clone()).collect(),
			_ => panic!("{:?} type given to GenUDT!", t),
		};
		// create an appropriate value for every possible type.
		let mut val: Vec<Box<Generator>> = Vec::new();
		for x in tys.iter() {
			let v = create(&x);
			val.push(v);
		}
		let nval: usize = val.len();
		assert_eq!(tys.len(), val.len());
		GenUDT{
			types: tys,
			values: val,
			// we need a vector of 0s the same size as 'values' or 'types'
			idx: (0..nval).map(|_| 0).collect(),
		}
	}
}

impl Generator for GenUDT {
	fn get(&self) -> String {
		use std::fmt::Write;
		let mut rv = String::new();
		write!(&mut rv, "{{\n").unwrap();

		for i in 0..self.values.len() {
			let nm = match self.types[i] {
				Type::Field(ref name, _) => name,
				ref x => panic!("GenUDT types are {:?}, not fields?", x),
			};
			write!(&mut rv, "\t\t.{} = {},\n", nm, self.values[i].get()).unwrap();
		}

		write!(&mut rv, "\t}}").unwrap();
		return rv;
	}

	// The number of states a UDT has is all possibilities of all fields.
	fn n_state(&self) -> usize {
		self.values.iter().fold(1, |acc, ref v| acc*v.n_state())
	}

	// We have an index for every field value.  It's sort-of an add-with-carry:
	// we try to add to the smallest integer, but when that overflows we jump to
	// the next field's index.
	// If we reset EVERY index, then we are actually at our end state and nothing
	// changes.
	fn next(&mut self) {
		for (i, v) in self.values.iter().enumerate() {
			if self.idx[i] < v.n_state()-1 {
				self.idx[i] = self.idx[i] + 1;
				return;
			}
			self.idx[i] = 0;
		}
		// if we got here, then we reset *everything*.  That means we were actually
		// done, and now we just accidentally reset all the indices to the default
		// state.  So here we re-reset them to the end state before returning.
		for (i, v) in self.values.iter().enumerate() {
			self.idx[i] = v.n_state()
		}
	}
}

pub struct GenPointer {
	cls: TC_Pointer,
	idx: usize,
}

impl GenPointer {
	pub fn create(_: &Type) -> Self {
		// doesn't seem to be a good way to assert that t is a &Type::Pointer...
		GenPointer{ cls: TC_Pointer::new(), idx: 0 }
	}
}

impl Generator for GenPointer {
	fn get(&self) -> String { self.cls.value(self.idx).to_string() }
	fn n_state(&self) -> usize { self.cls.n() }
	fn next(&mut self) {
		if self.idx < self.cls.n()-1 {
			self.idx = self.idx + 1
		}
	}
}

//---------------------------------------------------------------------

pub struct FreeEnum {
	pub name: String,
	pub tested: GenEnum,
	pub dest: Use,
	pub ty: Type,
}

impl Free for FreeEnum {
	fn typename(&self) -> String { return self.ty.name(); }
	fn name(&self) -> String { return self.name.clone(); }
	fn value(&self) -> String {
		return self.tested.get();
	}
}

#[allow(dead_code)]
pub struct FreeI32 {
	pub name: String,
	pub tested: GenI32,
	pub dest: Use,
	pub ty: Type,
}

impl Free for FreeI32 {
	fn typename(&self) -> String { return self.ty.name(); }
	fn name(&self) -> String { return self.name.clone(); }
	fn value(&self) -> String {
		return self.tested.get();
	}
}

pub struct FreeUDT {
	pub name: String,
	pub tested: GenUDT,
	pub dest: Use,
	pub ty: Type,
}

impl Free for FreeUDT {
	fn typename(&self) -> String { return self.ty.name(); }
	fn name(&self) -> String { return self.name.clone(); }
	fn value(&self) -> String { self.tested.get() }
}
