use crate::entity;
use crate::property;
use std::collections::HashMap;
use std::mem;
use std::rc::Rc;

#[derive(Clone)]
pub enum PropChange {
    Update(Rc<property::Value>),
    Delete(mem::Discriminant<property::Value>),
}

// Create and/or update an entity
#[derive(Clone)]
pub struct EntityChanges {
    pub ename: entity::Name,
    pub props: Vec<PropChange>,
}

impl EntityChanges {
    /// Add or replace a property
    pub fn add(&mut self, prop: property::Value) {
        self.props.push(PropChange::Update(Rc::new(prop)));
    }
    /// Remove a property from the entity
    pub fn delete(&mut self, prop: &property::Value) {
        self.props.push(PropChange::Delete(mem::discriminant(prop)));
    }
}

#[derive(Clone)]
pub enum Changes {
    /// Create or update entity
    Update(EntityChanges),
    /// Delete entity
    Delete(entity::Name),
    /// Mark this transaction as closed
    Closed,
}

pub struct Transaction {
    pub data: Vec<Changes>, // todo private
    /// name for create new object, available only if the transaction is active
    pub last_name: Option<entity::Name>,
}

impl Transaction {
    pub fn create_entity(&mut self) -> &mut EntityChanges {
        let name = self.last_name.as_ref().unwrap().clone();

        let v: &mut Vec<u32> = self.last_name.as_mut().unwrap();
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

    pub fn close(&mut self) {
        self.data.push(Changes::Closed);
    }

    pub fn merge(transactions: &Vec<Transaction>) -> Transaction {
        let mut res = Transaction {
            data: vec![],
            last_name: None,
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
                Changes::Closed => {}
            }
        }
        return changes_count;
    }
}

pub struct Storage {
    /// Transaction history for undo
    pub htrs: Vec<Transaction>,
}

pub struct TrStorages {
    storages: HashMap<property::DocId, Storage>,
}

impl TrStorages {
    pub fn new() -> Self {
        TrStorages {
            storages: HashMap::new(),
        }
    }

    pub fn get_or_open_transaction_storage<'x>(
        &'x mut self,
        doc_id: property::DocId,
    ) -> &'x Storage {
        self.storages.insert(doc_id, Storage { htrs: vec![] });
        if let Some(storage) = self.storages.get(&doc_id) {
            return storage;
        } else {
            unreachable!()
        }
    }
}
