// data entity

use crate::property::{self, Value2, KT};
use crate::transaction;
use crate::transaction::EntityChanges;
//use core::borrow;
use std::any::{Any, TypeId};
use std::borrow::Borrow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::io;
use std::io::{Error, ErrorKind, Read, Write};
use std::mem;
use std::rc::Rc;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Name of a document entity.
/// Documents may be nested one another; therefore, the name is a vector.
/// The name of the removed entity is reserved and will never be used again.
pub type Name = Vec<u32>; // use SmallVec smallvec::*; or possible store as one number

const CHG_CREATED: u32 = 1;
const CHG_DEL_PROP: u32 = 2;
const CHG_UPD_PROP: u32 = 4;
const CHG_ADD_PROP: u32 = 8;
const CHG_DELETED: u32 = 16;

/// Minimal (and initial) entity name
pub const START_NAME: u32 = 0;

pub struct Entity {
    /// Full entity name in the document
    pub name: Name,
    /// Properties assigned to this entity
    pub props2: Vec<Rc<property::Value2>>,
    /// If Some() this is inserted document, and it usually stores nested entities
    pub children: Option<Vec<Entity>>,
    // Keeps links to this from others
    // links: Vec<(Name, std::rc::Weak<dyn EntityUser>)>,
}

impl fmt::Debug for property::Value2 {
    // Required method
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //let ref b = self.value;

        //let val: dyn Any + 'static = b.borrow();

        write!(f, "  {}: ", self.key)?;

        //        let c = an.downcast_ref::<&dyn std::fmt::Display>();

        let a: &dyn Any = self.value.borrow();
        let c = a.downcast_ref::<&dyn std::fmt::Display>();

        //        writeln!(f, "{}", c.is_some())?;

        let aa: &dyn Any = &(1, 2) as &dyn Any;

        //let b:  = a.try_into();

        //        if let Some(f) = self.value.downcast_ref::<dyn std::fmt::Display>() {
        //        }

        //let an: &dyn std::any::Any = self.value.borrow();
        //writeln!(f, "{:?}", an)?;

        //        (&an as &dyn std::fmt::Debug).fmt(f);

        //let v: &std::fmt::Debug = an.into();

        //let v = an.downcast_ref::<std::fmt::Debug>();

        //let v = self.value.downcast_ref::<&dyn std::fmt::Display>();
        //writeln!(f, "{}", v.is_some())?;

        if self.value.is::<i32>() {
            writeln!(f, "{}", self.value.downcast_ref::<i32>().unwrap())?;
        } else if self.value.is::<&str>() {
            writeln!(f, "\"{}\"", self.value.downcast_ref::<&str>().unwrap())?;
        } else {
            self.value.fmt(f)?;
            //writeln!(f, " unsupported type!")?;
        }

        Ok(())
    }
}

impl fmt::Debug for Entity {
    // Required method
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Entity #{}:",
            self.name
                .iter()
                .map(|n| format!("{n}"))
                .collect::<Vec<String>>()
                .join(",")
        )?;
        for p in &self.props2 {
            writeln!(f, "  {}", p.key)?;
        }
        Ok(())
    }
}

impl Entity {
    pub fn get_property<T: Copy + 'static>(&self, key: property::KT) -> Option<T> {
        if let Some(pos) = self.props2.iter().position(|p| p.key == key) {
            if let Some(v) = self.props2[pos].value.downcast_ref::<T>() {
                return Some(v.clone());
            }
        }
        None
    }

    pub fn get_property_ptr(&self, key: property::KT) -> Option<Rc<Value2>> {
        if let Some(pos) = self.props2.iter().position(|p| p.key == key) {
            Some(self.props2[pos].clone())
        } else {
            None
        }
    }

    pub fn properties(&self) -> &Vec<Rc<Value2>> {
        &self.props2
    }

    fn get_child(&self, mut name_iter: core::slice::Iter<u32>) -> Option<&Entity> {
        match name_iter.next() {
            Some(&n) => {
                match &self.children {
                    None => {}
                    Some(chlds) => {
                        if let Some(pos) = chlds.iter().position(|e| *e.name.last().unwrap() == n) {
                            return chlds[pos].get_child(name_iter);
                        }
                    }
                };
                None
            }

            None => Some(self), // empty iterator, point at this object
        }
    }

    fn apply_changes(
        &mut self,
        changes: &transaction::PropChange,
        storages: &mut Vec<TransactionStorage>,
    ) -> Result<ChangedEntities, &'static str> {
        match changes {
            transaction::PropChange::Update(prop_ptr) => {
                //let prop_discr = mem::discriminant(prop_ptr.as_ref());
                if let Some(pos) = self.props2.iter().position(|p| p.key == prop_ptr.key) {
                    self.props2[pos] = prop_ptr.clone();
                    Ok(ChangedEntities::from(&self.name, CHG_UPD_PROP))
                } else {
                    self.props2.push(prop_ptr.clone());
                    let mut entity_changes = ChangedEntities::from(&self.name, CHG_ADD_PROP);

                    if prop_ptr.key == property::INS_DOC {
                        if let Some(doc_id) = prop_ptr.value.downcast_ref::<property::DocId>() {
                            // open inserted document and apply transaction from it to the children of this entity
                            let storage = Document::get_or_open_transactions(storages, *doc_id);
                            let mut content: Vec<Entity> = vec![];
                            let united_trs = transaction::Transaction::merge(&storage.htrs);
                            let changes = Document::apply_transaction_private(
                                &united_trs,
                                &mut content,
                                storages,
                            )?;
                            entity_changes.merge(changes);
                            self.children = Some(content);
                            //}
                        } else {
                            return Err("unexpected document id type");
                        }
                    }
                    Ok(entity_changes)
                }
            }

            transaction::PropChange::Delete(key) => {
                // removal of a property from an entity
                if let Some(pos) = self.props2.iter().position(|p| p.key == *key) {
                    self.props2.swap_remove(pos);
                } // all attempts to delete a non-existent property are ignored
                Ok(ChangedEntities::from(&self.name, CHG_DEL_PROP))
            }
        }
    }

    fn insert_document() {
        // TODO
    }
}

#[derive(Clone)]
pub struct PlainEntity {
    pub props: Vec<Rc<property::Value2>>,
}

impl PlainEntity {
    fn new(e: &Entity) -> Self {
        PlainEntity {
            props: e.props2.clone(),
        }
    }
}

// The objects of an application that uses this library should implement this trait.
//pub trait EntityUser {
//    fn on_change(&mut self, entity: &Entity, flags: u32) -> bool;
//    // ...
//}
//
//pub fn entity_user_factory(_entity: &Entity) -> Option<Rc<dyn EntityUser>> {
//    // detect required object type by the properties and create it
//    unimplemented!();
//}

pub struct ChangedEntities {
    // TODO use transaction::Changes instead
    pub data: HashMap<Name, u32>,
}

impl ChangedEntities {
    fn new() -> Self {
        ChangedEntities {
            data: HashMap::new(),
        }
    }

    fn from(name: &Name, flags: u32) -> Self {
        ChangedEntities {
            data: HashMap::from([(name.clone(), flags)]),
        }
    }

    fn add(&mut self, name: &Name, flags: u32) {
        if let Some(old_flags) = self.data.insert(name.clone(), flags) {
            self.data.insert(name.clone(), flags | old_flags);
        }
    }

    fn merge(&mut self, other: Self) {
        for (name, flags) in other.data {
            self.add(&name, flags);
        }
    }
}

// The history of document changes
struct TransactionStorage {
    id: property::DocId,
    htrs: Vec<transaction::Transaction>,
    /// How many transaction from `htrs` is currently applied to document.
    /// May be less than size of the vector in case of undo.
    applied: usize,
    /// Latest used name of entity created
    last_id: u32,
}

// The document opened in editor
pub struct Document {
    /// Document consist of the entities
    content: Vec<Entity>,

    /// The current active transaction to make changes to this document
    atrs: transaction::Transaction,

    /// History of changes made to this document
    my: TransactionStorage,

    /// Cache of all used documents
    other: Vec<TransactionStorage>,
}

impl Document {
    pub fn new(id: property::DocId) -> Self {
        Document {
            content: vec![],
            atrs: transaction::Transaction {
                data: vec![],
                last_id: Some(vec![START_NAME]),
            },
            my: TransactionStorage {
                id,
                htrs: vec![],
                applied: 0,
                last_id: START_NAME,
            },
            other: vec![],
        }
    }

    /// Change current document without destroying object
    pub fn switch(&mut self, id: property::DocId) -> Result<(), &'static str> {
        match self.other.iter().position(|h| id == h.id) {
            None => {
                self.other.push(mem::replace(
                    &mut self.my,
                    TransactionStorage {
                        id,
                        htrs: vec![],
                        applied: 0,
                        last_id: START_NAME,
                    },
                ));
            }

            Some(res) => {
                mem::swap(&mut self.other[res], &mut self.my);
            }
        };

        self.undo(0)?;

        self.atrs.last_id = Some(vec![self.my.last_id]);

        Ok(())
    }

    pub fn entities(&self, with_children: bool) -> EntityIterator {
        EntityIterator {
            index: vec![0],
            with_children,
            entities: &self.content,
        }
    }

    /// Find entity of (this or inserted) document
    pub fn get_entity(&self, name: Name) -> Option<&Entity> {
        let mut name_iter = name.iter();
        if let Some(&top_name) = name_iter.next() {
            if let Some(top_index) = self
                .content
                .iter()
                .position(|e| *e.name.last().unwrap() == top_name)
            {
                return self.content[top_index].get_child(name_iter);
            }
        }
        None
    }

    /// This convenient method is useful if all you need to do is read a property
    pub fn get_property<T: Copy + 'static>(&self, entity_name: Name, key: KT) -> Option<T> {
        if let Some(entity) = self.get_entity(entity_name) {
            entity.get_property::<T>(key)
        } else {
            None
        }
    }

    pub fn create_entity(&mut self) -> &mut EntityChanges {
        self.atrs.create_entity()
    }

    pub fn update_entity(&mut self, name: Name) -> &mut EntityChanges {
        self.atrs.update_entity(name)
    }

    pub fn delete_entity(&mut self, name: Name) {
        self.atrs.delete_entity(name)
    }

    /// Create and return copy of all the entities by its names.
    /// To make "cut" command, the entities must be deleted just after copying.
    pub fn copy(&self, names: BTreeSet<Name>) -> Vec<PlainEntity> {
        // TODO if copied two entity and the first has link to the second, the link should be updated
        self.entities(true)
            .filter_map(|e| {
                if names.contains(&e.name) {
                    Some(PlainEntity::new(e))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Create in the document copies of entities previously copied with Document::copy.
    /// The document own all the created entity, even if it was taken from any inserted document.
    pub fn paste(&mut self, ref clipboard: Vec<PlainEntity>) {
        let ref mut trs = self.atrs;
        for entity in clipboard {
            let changes = trs.create_entity();
            for prop in &entity.props {
                changes.copy(prop.clone());
            }
        }
    }

    pub fn history_size(&self) -> (usize, usize) {
        (self.my.htrs.len(), self.my.applied)
    }

    pub fn undo(&mut self, delta: isize) -> Result<(), &'static str> {
        let new_pos: usize = (self.my.applied as isize + delta) as usize;
        if self.my.htrs.len() < new_pos {
            return Err("undo history overflow");
        }
        if -delta > self.my.applied as isize {
            return Err("undo history underflow");
        }

        self.content.clear();
        self.my.applied = 0;
        for i in 0..new_pos {
            Document::apply_transaction_private(
                &self.my.htrs[i],
                &mut self.content,
                &mut self.other,
            )?;
            self.my.applied += 1;
        }

        Ok(())
    }

    /// Apply all the modifications accumulated in the active transaction to the document and start a new transaction.
    pub fn commit_transaction(&mut self) -> Result<ChangedEntities, &'static str> {
        let changes = self.apply_transaction()?;

        // save back to document last used entity name
        if let Some(trs_last_id) = &self.atrs.last_id {
            if let Some(id) = trs_last_id.last() {
                self.my.last_id = *id;
            }
        }

        // archive the finished transaction and create new
        let finished = mem::replace(
            &mut self.atrs,
            transaction::Transaction {
                data: vec![],
                last_id: Some(vec![self.my.last_id]),
            },
        );
        self.my.htrs.truncate(self.my.applied);
        self.my.htrs.push(finished);
        self.my.applied = self.my.htrs.len();

        Ok(changes)
    }

    /// Applying without committing is only permitted for specific kinds of modifications
    pub fn apply_transaction(&mut self) -> Result<ChangedEntities, &'static str> {
        Document::apply_transaction_private(&self.atrs, &mut self.content, &mut self.other)
    }

    fn apply_transaction_private(
        trs: &transaction::Transaction,
        content: &mut Vec<Entity>,
        inserted_storages: &mut Vec<TransactionStorage>,
    ) -> Result<ChangedEntities, &'static str> {
        let mut entity_changes = ChangedEntities::new();
        for item in &trs.data {
            match &item {
                transaction::Changes::Update(changes) => {
                    let chgs = Document::entity_create_or_update(
                        changes.ename.iter(),
                        &changes.props,
                        content,
                        inserted_storages,
                    )?;
                    entity_changes.merge(chgs);
                }
                transaction::Changes::Delete(name) => {
                    if let Some(last_name) = name.last() {
                        let entity_pos = content
                            .iter()
                            .position(|e| *e.name.last().unwrap() == *last_name);
                        if let Some(pos) = entity_pos {
                            content.swap_remove(pos);
                            entity_changes.add(&name, CHG_DELETED);
                            return Ok(entity_changes);
                        }
                    }
                    return Err("no suitable object was found");
                }
            }
        }
        Ok(entity_changes)
    }

    fn get_or_open_transactions<'y>(
        history: &'y mut Vec<TransactionStorage>,
        id: property::DocId,
    ) -> &'y TransactionStorage {
        match history.iter().position(|h| id == h.id) {
            None => {
                history.push(TransactionStorage {
                    id,
                    htrs: vec![],
                    applied: 0,
                    last_id: START_NAME,
                });
                &history.last().unwrap()
            }

            Some(res) => &history[res],
        }
    }

    fn entity_create_or_update(
        mut ename: std::slice::Iter<u32>,
        props: &Vec<transaction::PropChange>,
        content: &mut Vec<Entity>,
        storages: &mut Vec<TransactionStorage>,
    ) -> Result<ChangedEntities, &'static str> {
        if ename.len() > 1 {
            // for nested entity call this method recursively
            let &last_name = ename.next().unwrap();
            for entity in content.iter_mut() {
                if *entity.name.last().unwrap() == last_name {
                    if let Some(chlds) = &mut entity.children {
                        return Self::entity_create_or_update(ename, props, chlds, storages);
                    }
                    return Err("trying to change a child of an entity without children");
                }
            }
            return Err("entity not found");
        }

        assert_eq!(ename.len(), 1);
        let mut object: Option<&mut Entity> = None;
        if let Some(&last_name) = ename.last() {
            for entity in content.iter_mut() {
                // search for the entity by the name specified
                if *entity.name.last().unwrap() == last_name {
                    object = Some(entity);
                    break;
                }
            }

            let mut entity_changes = ChangedEntities::new();
            if object.is_none() {
                // entity with specified name isn't found, create new
                content.push(Entity {
                    name: vec![last_name],
                    props2: vec![],
                    children: None,
                    //links: vec![],
                });
                object = content.last_mut();

                if let Some(obj) = &object {
                    entity_changes.add(&obj.name, CHG_CREATED);
                }
            }

            if let Some(entity) = object {
                // create, change and delete the properties of the entity

                for prop_change in props {
                    let chg = entity.apply_changes(prop_change, storages)?;
                    entity_changes.merge(chg);
                }
                return Ok(entity_changes);
            }
        }

        Err("no suitable object was found")
    }
}

pub struct EntityIterator<'a> {
    index: Vec<usize>,
    with_children: bool,
    entities: &'a Vec<Entity>,
}

impl<'a> EntityIterator<'a> {
    fn get_entity(&self, mut indexes: std::slice::Iter<usize>) -> Option<&'a Entity> {
        if let Some(&first_pos) = indexes.next() {
            if self.entities.len() > first_pos {
                let mut res = &self.entities[first_pos];

                for &pos in indexes {
                    if let Some(chlds) = &res.children {
                        if let Some(entity) = chlds.get(pos) {
                            res = entity;
                            continue;
                        }
                    }
                    return None;
                }
                return Some(res);
            }
        }
        return None;
    }
}

impl<'a> Iterator for EntityIterator<'a> {
    type Item = &'a Entity;

    fn next(&mut self) -> Option<&'a Entity> {
        let mut res = self.get_entity(self.index.iter());

        // increment index
        if res.is_some() {
            if self.with_children && res.unwrap().children.is_some() {
                self.index.push(0);
            } else {
                *self.index.last_mut().unwrap() += 1;
            }
        } else {
            // if none found, go to the parent's entities
            while res.is_none() {
                self.index.pop();
                if !self.index.is_empty() {
                    *self.index.last_mut().unwrap() += 1;
                    res = self.get_entity(self.index.iter());

                    if res.is_some() {
                        if res.unwrap().children.is_some() {
                            self.index.push(0);
                        } else {
                            *self.index.last_mut().unwrap() += 1;
                        }
                    }
                } else {
                    break;
                }
            }
        }

        return res;
    }
}

pub struct PropertyIterator<'a> {
    index: usize,
    properties: &'a Vec<Rc<property::Value2>>,
}

impl<'a> Iterator for PropertyIterator<'a> {
    type Item = &'a property::Value2;

    fn next(&mut self) -> Option<&'a property::Value2> {
        if self.index < self.properties.len() {
            self.index += 1;
            Some(&self.properties[self.index - 1])
        } else {
            None
        }
    }
}

#[test]
fn trs_len_grow() {
    let mut doc = crate::entity::Document::new(1);

    const KEY1: property::KT = 0;
    const KEY2: property::KT = 0;

    let l0 = doc.atrs.len();
    doc.create_entity().add(KEY1, 101);
    let l1 = doc.atrs.len();
    assert!(l0 < l1);

    doc.create_entity().add(KEY1, 101).add(KEY2, "qwerty");
    let l2 = doc.atrs.len();
    assert!(l1 < l2);
    assert!(l1 - l0 < l2 - l1);

    //doc.trs().close();
    //assert_eq!(doc.trs().len(), l2);
}
