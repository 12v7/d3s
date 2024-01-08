

// property of a data entity
// Property is a key-value pair. Key has type of &str. Value is string, guid, number, binary, or another type of data.

use std::boxed::Box;
use std::any::Any;
//use std::fmt::Debug;
//use serde::{Serialize, Deserialize};

pub type DocId = u32;

// A property linked to an entity
// A property can store a link to another object to make a graph relationship.
//pub struct Link {
//  target: crate::entity::Name,
//  objects: Vec<crate::entity::Name>,
//}


// Property value
// Stored as Rc<Property>, may be assigned to several entiny.

// property key type, &str by default
// using an interger number as the type of keys can slightly improve memory consumption
//pub type KT = &'static str;

//pub const INS_DOC: KT = "insDoc";

pub type KT = u32;
//pub const COLOR: KT = 11;
//pub const TITLE: KT = 22;
pub const INS_DOC: KT = 33;


//#[derive(Debug)]

//#[derive(Serialize, Deserialize)] 
pub struct Value2 { // KVT<T> {
    pub key: KT,
    pub value: Box<dyn Any>,
}
