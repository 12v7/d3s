#![allow(dead_code)]

pub mod entity;
pub mod property;
pub mod transaction;

#[cfg(test)]
mod storage;

#[cfg(test)]
mod tests {

    use crate::{storage::*, property};
    use std::{borrow::Cow, str::Bytes};
    use quick_protobuf::MessageWrite;
    use std::rc::Rc;

/// [protobuf docs](https://protobuf.dev/programming-guides/proto3/)
/// [chosen library](https://github.com/tafia/quick-protobuf)


//impl<'x> PbPropertyValue<'x> {
//    fn from_pv(pv: Rc<property::Value2>) -> Self
//    {
//        //let b = pv.value.as_bytes();
//        //let b = pv.value.downcast_ref::<Bytes>();
//        let b: &str = "pv.value.into()";
//
//        //let b = b.unwrap();
//        //let b = b.iter().collect::<Vec<u8>>();
//        PbPropertyValue {
//            key: Cow::Borrowed(pv.key),
//            value: Cow::Borrowed(b.as_bytes()),
//        }
//    }
//}

#[test]
fn protobuf() {


//    let pvptr = Rc::new(pv1);
//
//    let pv2 = PbPropertyValue::from_pv(pvptr);
//
//    let pv = PbPropertyValue {
//        key: Cow::Borrowed("key"),
//        value: Cow::Borrowed(b"value"),
//    };
//
//    
//    
//    assert_eq!(pv.get_size(), 12);

//    PropertyValue::
}


}
