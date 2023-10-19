// property of data entity (examples)

pub type DocId = u32;

/// A property linked to an entity
/// A property can store a link to another object to make a graph relationship.
//pub struct Link {
//  target: crate::entity::Name,
//  objects: Vec<crate::entity::Name>,
//}

/// Property value
/// Stored as Rc<Property>, may be assigned to several entiny.
#[repr(isize)]
#[derive(PartialEq, Debug)]
pub enum Value {
    Title(String), // title of document, symbol or something else
    Color(u32), // RGB color
    InsDoc(DocId), // inserted document identificator
//    Pos(u32, u32), // point coordinates
}

pub const COLOR: &Value = &Value::Color(u32::MAX);
pub const TITLE: &Value = &Value::Title(String::new());
pub const INS_DOC: &Value = &Value::InsDoc(DocId::MAX);
//pub const POS: &Value = &Value::Pos(u32::MAX, u32::MAX);
