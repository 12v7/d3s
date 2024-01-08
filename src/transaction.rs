use crate::entity;
use crate::property;
use std::any::Any;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;
use std::rc::Rc;

struct TypeRegistryItem {
    create: fn(r: &mut dyn Read) -> Option<Box<dyn Any>>,
    store: fn(&Box<dyn Any>, r: &mut dyn Write) -> Result<(), ()>,
}

struct TypeRegistry {
    // the type of value always defined by key
    all: HashMap<property::KT, TypeRegistryItem>,
}

//trait Storable {
//    fn save(&self, writer: &mut dyn Write) -> io::Result<()>;
//    fn load<T>(reader: &mut dyn Read) -> Result<T, io::Error>;
//}

impl TypeRegistry {
    fn read_key(r: &mut dyn Read) -> Result<property::KT, Error> {
        //let mut str_len_buf = [0u8; 1];
        //r.read_exact(&mut str_len_buf)?;
        //let str_len = str_len_buf[0] as usize + 1;
        //let mut str_buf = Vec::<u8>::with_capacity(str_len);
        //str_buf.resize(str_len, 0);
        //r.read_exact(&mut str_buf)?;
        //String::from_utf8(str_buf).map_err(|_| Error::from(ErrorKind::InvalidData))

        let mut bytes = [0u8; 4];
        if r.read_exact(&mut bytes).is_ok() {
            Ok(property::KT::from_le_bytes(bytes))
        } else {
            Err(Error::from(ErrorKind::InvalidData))
        }
    }
    fn write_key(key: property::KT, w: &mut dyn Write) -> io::Result<()> {
        //let len = key.as_bytes().len();
        //match len {
        //    1..=256 => {
        //        w.write_all(&[(len - 1usize) as u8])?;
        //        w.write_all(key.as_bytes())
        //    }
        //    _ => Err(Error::from(ErrorKind::InvalidData)),
        //}
        w.write_all(&key.to_le_bytes())
    }
}

#[derive(Clone)]
pub enum PropChange {
    Update(Rc<property::Value2>),
    Delete(property::KT),
}

// Create and/or update an entity
#[derive(Clone)]
pub struct EntityChanges {
    pub ename: entity::Name,
    pub props: Vec<PropChange>,
}

impl EntityChanges {
    fn save(&self, w: &mut dyn Write) -> io::Result<()> {
        let types = TypeRegistry {
            all: HashMap::new(),
        };

        w.write_all(&self.props.len().to_be_bytes())?;

        for prop_change in &self.props {
            match prop_change {
                PropChange::Update(rc_value) => {
                    w.write_all(&[1])?;
                    if let Some(td) = types.all.get(&rc_value.key) {
                        TypeRegistry::write_key(rc_value.key, w)?;
                        (td.store)(&rc_value.value, w);
                    }
                }
                PropChange::Delete(key) => {
                    w.write_all(&[0])?;
                    TypeRegistry::write_key(*key, w)?;
                }
            }
        }
        //let id = &value.type_id();
        //let mut hasher = DefaultHasher::new();
        //id.hash(&mut hasher);
        //let h = hasher.finish();
        Ok(())
    }

    fn load(&mut self, r: &mut dyn Read) -> io::Result<()> {
        let types = TypeRegistry {
            all: HashMap::new(),
        };

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        let count = usize::from_be_bytes(buf);

        //        let h = u64::from_be_bytes(buf);

        for _ in 0..count {
            let mut buf= [0u8; 1];
            r.read_exact(&mut buf)?;

            let key = TypeRegistry::read_key(r)?;
            //let key = key_str.as_str();
            //String::from_utf8(str_buf).map_err(|_| Error::from(ErrorKind::InvalidData))
            match buf[0] {
                0 => self.props.push(PropChange::Delete(key)),
                1 => {
                    if let Some(td) = types.all.get(&key) {
                        let value = (td.create)(r).ok_or(Error::from(ErrorKind::InvalidData))?;
                        let pv = property::Value2 { key, value };
                        self.props.push(PropChange::Update(Rc::new(pv)));
                    }
                }
                _ => {} //{ return Err(Error::from(ErrorKind::InvalidData)); }
            }
        }

        //if let Some(td) = types.all.get(&h) {
        //    let key = TypeRegistry::read_key(r)?.as_str();
        //    let value = (td.create)(&mut r);
        //    e.props2.push(Rc::new(property::Value2 { key, value }));
        //}
        Ok(())
    }

    /// Add or replace a property
    pub fn add<T: Any>(&mut self, key: property::KT, value: T) -> &mut Self {
        self.props
            .push(PropChange::Update(Rc::new(property::Value2 {
                key: key,
                value: Box::new(value),
            })));
        self
    }

    /// Remove a property from the entity
    pub fn delete(&mut self, key: property::KT) -> &mut Self {
        self.props.push(PropChange::Delete(key));
        self
    }

    /// Copy a property from one entity to the other.
    /// The method designed for handling huge properties, such as images.
    pub fn copy(&mut self, ref from: Rc<property::Value2>) -> &mut Self {
        self.props.push(PropChange::Update(from.clone()));
        self
    }
}

#[derive(Clone)]
pub(crate) enum Changes {
    /// Create or update entity
    Update(EntityChanges),
    /// Delete entity
    Delete(entity::Name),
}

pub(crate) struct Transaction {
    pub data: Vec<Changes>,
    /// name for create new object, available only if the transaction is active
    pub last_id: Option<entity::Name>,
}

impl Transaction {
    pub fn create_entity(&mut self) -> &mut EntityChanges {
        let name = self.last_id.as_ref().unwrap().clone();

        let v: &mut Vec<u32> = self.last_id.as_mut().unwrap();
        *v.last_mut().unwrap() += 1;

        self.update_entity(name)
    }

    pub fn update_entity(&mut self, name: entity::Name) -> &mut EntityChanges {
        self.data.push(Changes::Update(EntityChanges {
            ename: name,
            props: vec![],
        }));
        let changes = self.data.last_mut().unwrap();
        if let Changes::Update(chgs) = changes {
            chgs
        } else {
            unreachable!()
        }
    }

    pub fn delete_entity(&mut self, name: entity::Name) {
        self.data.push(Changes::Delete(name));
    }

    pub fn merge(transactions: &Vec<Transaction>) -> Transaction {
        let mut res = Transaction {
            data: vec![],
            last_id: None,
        };

        //    let mut iter: Option<core::slice::Iter<Changes>> = None;
        for trs in transactions {
            res.data.append(&mut trs.data.clone());

            //      if iter.is_none() {
            //        iter = Some(trs.data.iter());
            //      } else {
            //        iter.unwrap().chain(trs.data.iter());
            //      }
            // to do sort changes properly
        }

        return res;
    }

    // count all the changes in the transaction, useful for detect new changes
    pub fn len(&self) -> usize {
        let mut changes_count = 0;
        for data_item in &self.data {
            match data_item {
                Changes::Update(changes) => {
                    changes_count += changes.props.len();
                }
                Changes::Delete(_) => {
                    changes_count += 1;
                }
            }
        }
        return changes_count;
    }
}
