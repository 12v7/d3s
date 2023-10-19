// data entity

use crate::property;
use crate::transaction;
use std::collections::HashMap;
use std::mem;
use std::rc::Rc;

/// Name of an document entity.
/// Documents may be nested one another; therefore, the name is vector.
/// The name of the removed entity is reserved and will never be used again.
pub type Name = Vec<u32>; // use SmallVec smallvec::*; or possible store as one number

pub const CHG_CREATED: u32 = 1;
pub const CHG_DEL_PROP: u32 = 2;
pub const CHG_UPD_PROP: u32 = 4;
pub const CHG_ADD_PROP: u32 = 8;
pub const CHG_DELETED: u32 = 16;

/// Minimal entity name
pub const START_NAME: u32 = 0;

pub struct Entity {
    /// Full entity name in the document
    pub name: Name,
    /// Properties assigned to this entity
    pub props: Vec<Rc<property::Value>>,
    /// Inserted document if any
    pub children: Option<Vec<Entity>>,

    // Keeps links to this from others
    // links: Vec<(Name, std::rc::Weak<dyn EntityUser>)>,
}

impl Entity {
    pub fn properties(&self) -> PropertyIterator {
        PropertyIterator {
            index: 0,
            properties: &self.props,
        }
    }

    pub fn get_property(
        &self,
        property_type: &'static property::Value,
    ) -> Option<&property::Value> {
        let discr: mem::Discriminant<property::Value> = mem::discriminant(property_type);
        if let Some(pos) = self
            .props
            .iter()
            .position(|p| mem::discriminant(p.as_ref()) == discr)
        {
            Some(self.props[pos].as_ref())
        } else {
            None
        }
    }

    pub fn get_property_rc(
        &self,
        property_type: &'static property::Value,
    ) -> Option<Rc<property::Value>> {
        let discr: mem::Discriminant<property::Value> = mem::discriminant(property_type);
        if let Some(pos) = self
            .props
            .iter()
            .position(|p| mem::discriminant(p.as_ref()) == discr)
        {
            Some(self.props[pos].clone())
        } else {
            None
        }
    }

    pub fn get_children_entity(&self, mut name_iter: core::slice::Iter<u32>) -> Option<&Entity> {
        match name_iter.next() {
            Some(&n) => {
                match &self.children {
                    None => {}
                    Some(chlds) => {
                        if let Some(pos) = chlds.iter().position(|e| *e.name.last().unwrap() == n) {
                            return chlds[pos].get_children_entity(name_iter);
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
        storages: &mut Vec<History>,
    ) -> Result<EnitiyChanges, &'static str> {
        match changes {
            transaction::PropChange::Update(prop_ptr) => {
                let prop_discr = mem::discriminant(prop_ptr.as_ref());
                if let Some(pos) = self
                    .props
                    .iter()
                    .position(|p| mem::discriminant(p.as_ref()) == prop_discr)
                {
                    self.props[pos] = prop_ptr.clone();
                    Ok(EnitiyChanges::from(&self.name, CHG_UPD_PROP))
                } else {
                    self.props.push(prop_ptr.clone());
                    let mut entity_changes = EnitiyChanges::from(&self.name, CHG_ADD_PROP);

                    if prop_discr == mem::discriminant(property::INS_DOC) {
                        if let &property::Value::InsDoc(doc_id) = prop_ptr.as_ref() {
                            // open inserted document and apply transaction from it to the children of this entity

                            let storage = Document::get_or_open_transactions(storages, doc_id);
                            let mut content: Vec<Entity> = vec![];
                            let united_trs = transaction::Transaction::merge(&storage.htrs);
                            let changes = Document::apply_transaction_private(
                                &united_trs,
                                &mut content,
                                storages,
                            )?;
                            entity_changes.merge(changes);
                            self.children = Some(content);
                        }
                    }
                    Ok(entity_changes)
                }
            }

            transaction::PropChange::Delete(discr) => {
                // removal of a property from an entity
                if let Some(pos) = self
                    .props
                    .iter()
                    .position(|p| mem::discriminant(p.as_ref()) == *discr)
                {
                    self.props.swap_remove(pos);
                } // ignore the attempt to delete a non-existent property
                Ok(EnitiyChanges::from(&self.name, CHG_DEL_PROP))
            }
        }
    }
}

/// The objects of an application that uses this library should implement this trait.
pub trait EntityUser {
    fn on_change(&mut self, entity: &Entity, flags: u32) -> bool;
    // ...
}

pub struct EnitiyChanges {
    // TODO rename, this name already used in crate::transaction::EntityChanges;
    pub data: HashMap<Name, u32>,
}

impl EnitiyChanges {
    fn new() -> Self {
        EnitiyChanges {
            data: HashMap::new(),
        }
    }

    fn from(name: &Name, flags: u32) -> Self {
        EnitiyChanges {
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

pub fn entity_user_factory(_entity: &Entity) -> Option<Rc<dyn EntityUser>> {
    // detect required object type by the properties and create it
    unimplemented!();
}

// The history of document changes
pub struct History {
    pub id: property::DocId,
    pub htrs: Vec<transaction::Transaction>,
    /// How many transaction from `htrs` is applied to document.
    /// May be less than size of htrs in case of undo.
    pub applied: usize,
    /// Latest used name of entity created
    last_name: u32,
}

// The document opened in editor
pub struct Document {
    /// Document consist of entities
    content: Vec<Entity>,

    /// Current active transaction for change this document
    pub atrs: transaction::Transaction,

    /// History of changes made to this document
    my: History,
    other: Vec<History>,
}

impl Document {
    pub fn new(doc_id: property::DocId) -> Self {
        Document {
            content: vec![],
            atrs: transaction::Transaction {
                data: vec![],
                last_name: Some(vec![START_NAME]),
            },
            my: History {
                id: doc_id,
                htrs: vec![],
                applied: 0,
                last_name: START_NAME,
            },
            other: vec![], //transaction::TrStorages::new(),
        }
    }

    /// Change current document without destroying object
    pub fn switch(&mut self, doc_id: property::DocId) -> Result<(), &'static str> {
        match self.other.iter().position(|h| doc_id == h.id) {
            None => {
                self.other.push(mem::replace(
                    &mut self.my,
                    History {
                        id: doc_id,
                        htrs: vec![],
                        applied: 0,
                        last_name: START_NAME,
                    },
                ));
            }

            Some(res) => {
                mem::swap(&mut self.other[res], &mut self.my);
            }
        };

        self.undo(0)?;

        self.atrs.last_name = Some(vec![self.my.last_name]);

        Ok(())
    }

    /// Find entity of (this or inserted) document
    pub fn get_entity(&self, name: &crate::entity::Name) -> Option<&Entity> {
        let mut name_iter = name.iter();
        if let Some(&top_name) = name_iter.next() {
            if let Some(top_index) = self
                .content
                .iter()
                .position(|e| *e.name.last().unwrap() == top_name)
            {
                return self.content[top_index].get_children_entity(name_iter);
            }
        }
        None
    }

    /// Get current active transaction to make a changes in the document
    pub fn trs(&mut self) -> &mut transaction::Transaction {
        &mut self.atrs
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

    /// Applying without commit allowed only for transactions of limited types
    pub fn apply_transaction<'a>(
        &mut self,
        //and_commit: bool,
    ) -> Result<EnitiyChanges, &'static str> {
        let entity_changes =
            Document::apply_transaction_private(&self.atrs, &mut self.content, &mut self.other)?;

        // TODO apply transaction partially for each occurence of Changes::Closed
        let mut and_commit = false;
        for d in &self.atrs.data {
            if let crate::transaction::Changes::Closed = *d {
                and_commit = true;
                break;
            }
        }

        if and_commit {
            if let Some(trs_last_name) = &self.atrs.last_name {
                self.my.last_name = *trs_last_name.last().unwrap();

                // move the transaction to the history
                let oldtrs = mem::replace(
                    &mut self.atrs,
                    transaction::Transaction {
                        data: vec![],
                        last_name: Some(vec![self.my.last_name]),
                    },
                );
                self.my.htrs.truncate(self.my.applied);
                self.my.htrs.push(oldtrs);
                self.my.applied = self.my.htrs.len();
            }
        }
        Ok(entity_changes)
    }

    fn apply_transaction_private<'a>(
        trs: &transaction::Transaction,
        content: &mut Vec<Entity>,
        inserted_storages: &mut Vec<History>,
    ) -> Result<EnitiyChanges, &'static str> {
        let mut entity_changes = EnitiyChanges::new();
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
                transaction::Changes::Closed => {}
            }
        }
        Ok(entity_changes)
    }

    fn get_or_open_transactions<'x>(
        history: &'x mut Vec<History>,
        doc_id: property::DocId,
    ) -> &'x History {
        match history.iter().position(|h| doc_id == h.id) {
            None => {
                history.push(History {
                    id: doc_id,
                    htrs: vec![],
                    applied: 0,
                    last_name: START_NAME,
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
        storages: &mut Vec<History>,
    ) -> Result<EnitiyChanges, &'static str> {
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

            let mut entity_changes = EnitiyChanges::new();
            if object.is_none() {
                // entity with specified name isn't found, create new
                content.push(Entity {
                    name: vec![last_name],
                    props: vec![],
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

    pub fn entities(&self) -> EntityIterator {
        EntityIterator {
            index: 0,
            entities: &self.content,
        }
    }
}

pub struct EntityIterator<'a> {
    index: usize,
    entities: &'a Vec<Entity>,
}

impl<'a> Iterator for EntityIterator<'a> {
    type Item = &'a Entity;

    fn next(&mut self) -> Option<&'a Entity> {
        if self.index < self.entities.len() {
            self.index += 1;
            Some(&self.entities[self.index - 1])
        } else {
            None
        }
    }
}

pub struct PropertyIterator<'a> {
    index: usize,
    properties: &'a Vec<Rc<property::Value>>,
}

impl<'a> Iterator for PropertyIterator<'a> {
    type Item = &'a property::Value;

    fn next(&mut self) -> Option<&'a property::Value> {
        if self.index < self.properties.len() {
            self.index += 1;
            Some(&self.properties[self.index - 1])
        } else {
            None
        }
    }
}
