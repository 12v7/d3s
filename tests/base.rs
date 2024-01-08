use std::collections::BTreeSet;

use d3s::entity::{Document, START_NAME};
use d3s::property::{DocId, INS_DOC, KT};
//use d3s::tr;

pub const COLOR: KT = 101; //"color";
pub const TITLE: KT = 102; //"title";

#[test]
fn create() {
    let mut doc = Document::new(1);

    doc.create_entity().add(COLOR, 123).add(TITLE, "qwerty");
    assert!(doc.commit_transaction().is_ok());

    assert_eq!(doc.entities(false).count(), 1);

    let e = doc.entities(false).next().unwrap();
    assert_eq!(e.properties().len(), 2);

    assert_eq!(e.get_property::<i32>(COLOR).unwrap(), 123);
    assert_eq!(e.get_property::<&str>(TITLE).unwrap(), "qwerty");
}

#[test]
fn update() {
    let mut doc = Document::new(1);

    doc.create_entity().add(COLOR, 101);
    assert!(doc.commit_transaction().is_ok());

    doc.update_entity(vec![START_NAME]).add(COLOR, 11);
    assert!(doc.commit_transaction().is_ok());

    assert_eq!(doc.entities(false).count(), 1);
    assert_eq!(
        doc.get_property::<i32>(vec![START_NAME], COLOR).unwrap(),
        11
    );
}

#[test]
fn create_two() {
    let mut doc = Document::new(1);

    doc.create_entity().add(COLOR, 101).add(TITLE, "qwerty");
    assert!(doc.apply_transaction().is_ok());

    doc.create_entity().add(TITLE, "qwerty");
    assert!(doc.commit_transaction().is_ok());

    assert_eq!(doc.entities(false).count(), 2);
}

#[test]
fn duplicate_property() {
    let mut doc = Document::new(1);

    doc.create_entity().add(TITLE, "qwerty");
    assert!(doc.commit_transaction().is_ok());

    doc.create_entity().add(TITLE, "yuiop");
    assert!(doc.commit_transaction().is_ok());

    // copy the property from the second entity to the first
    let ptr = doc
        .get_entity(vec![START_NAME + 1])
        .unwrap()
        .get_property_ptr(TITLE)
        .unwrap();
    doc.update_entity(vec![START_NAME]).copy(ptr);
    assert!(doc.commit_transaction().is_ok());

    assert_eq!(
        doc.get_property::<&str>(vec![START_NAME], TITLE).unwrap(),
        "yuiop"
    );
    assert_eq!(
        doc.get_property::<&str>(vec![START_NAME + 1], TITLE)
            .unwrap(),
        "yuiop"
    );
}

#[test]
fn delete_entity() {
    let mut doc = Document::new(1);

    doc.create_entity().add(COLOR, 101);
    assert!(doc.commit_transaction().is_ok());
    assert_eq!(doc.entities(false).count(), 1);

    doc.delete_entity(vec![START_NAME]);
    assert!(doc.commit_transaction().is_ok());
    assert_eq!(doc.entities(false).count(), 0);
}

#[test]
fn delete_property() {
    let mut doc = Document::new(1);

    let ce = doc.create_entity();
    ce.add(COLOR, 101);
    let name = ce.ename.clone();
    assert!(doc.commit_transaction().is_ok());
    assert!(!doc
        .get_entity(name.clone())
        .unwrap()
        .properties()
        .is_empty());

    doc.update_entity(name.clone()).delete(COLOR);
    assert!(doc.commit_transaction().is_ok());
    assert!(doc.get_entity(name).unwrap().properties().is_empty());
}

#[test]
fn undo_redo() {
    let mut doc = Document::new(1);
    assert_eq!(doc.history_size(), (0, 0));
    assert!(doc.undo(-1).is_ok() == false);

    {
        doc.create_entity().add(COLOR, 101);
        assert!(doc.commit_transaction().is_ok());
    }
    assert_eq!(doc.history_size(), (1, 1));

    {
        doc.update_entity(vec![START_NAME]).add(COLOR, 102);
        assert!(doc.commit_transaction().is_ok());
    }
    assert_eq!(doc.history_size(), (2, 2));

    assert!(doc.undo(-1).is_ok());
    assert_eq!(
        doc.get_property::<i32>(vec![START_NAME], COLOR).unwrap(),
        101
    );
    assert_eq!(doc.history_size(), (2, 1));

    assert!(doc.undo(1).is_ok());
    assert_eq!(
        doc.get_property::<i32>(vec![START_NAME], COLOR).unwrap(),
        102
    );
    assert_eq!(doc.history_size(), (2, 2));
    assert!(doc.undo(1).is_ok() == false);
}

#[test]
fn name_always_unique() {
    let mut doc = Document::new(1);

    doc.create_entity();
    assert!(doc.commit_transaction().is_ok());
    let name_0 = doc.entities(false).next().unwrap().name.clone();

    doc.delete_entity(name_0.clone());
    assert!(doc.commit_transaction().is_ok());

    doc.create_entity();
    assert!(doc.commit_transaction().is_ok());
    assert_eq!(doc.entities(false).count(), 1);
    // name of the removed entity is reserved and will never be used again
    assert!(name_0 != doc.entities(false).next().unwrap().name);
}

#[test]
fn reapply() {
    let mut doc = Document::new(1);
    {
        doc.create_entity().add(COLOR, 101);
    }
    assert!(doc.apply_transaction().is_ok());
    {
        doc.update_entity(vec![START_NAME]).add(COLOR, 202);
    }
    {
        let e = doc.entities(false).next().unwrap();
        assert_eq!(e.properties().len(), 1);
        assert_eq!(e.get_property::<i32>(COLOR).unwrap(), 101);
    }
    assert!(doc.apply_transaction().is_ok());
    {
        let e = doc.entities(false).next().unwrap();
        assert_eq!(e.get_property::<i32>(COLOR).unwrap(), 202);
    }
}

#[test]
fn switch_document() {
    let mut doc = Document::new(111);
    doc.create_entity().add(COLOR, 1);
    assert!(doc.commit_transaction().is_ok());

    assert!(doc.switch(222).is_ok());
    doc.create_entity().add(COLOR, 2);
    doc.create_entity().add(COLOR, 3);
    assert!(doc.commit_transaction().is_ok());
    assert_eq!(doc.entities(false).count(), 2);

    assert!(doc.switch(111).is_ok());
    assert_eq!(doc.entities(false).count(), 1);
}

#[test]
fn insert_document() {
    let mut doc = Document::new(222);
    doc.create_entity().add(COLOR, 2);
    assert!(doc.commit_transaction().is_ok());

    assert!(doc.switch(111).is_ok());
    doc.create_entity().add(INS_DOC, 222 as DocId);
    assert!(doc.commit_transaction().is_ok());

    assert_eq!(doc.entities(false).count(), 1);
    assert_eq!(doc.entities(true).count(), 2);
    let e = doc.entities(false).next().unwrap();

    assert_eq!(e.properties().len(), 1);
    assert_eq!(e.children.as_ref().unwrap().len(), 1);
}

/// Insert a document into another twice
#[test]
fn insert_docs_parallel() {
    let mut doc = Document::new(222);
    doc.create_entity().add(COLOR, 2);
    assert!(doc.commit_transaction().is_ok());

    assert!(doc.switch(111).is_ok());
    doc.create_entity().add(INS_DOC, 222 as DocId);
    doc.create_entity().add(INS_DOC, 222 as DocId);
    assert!(doc.commit_transaction().is_ok());

    assert_eq!(doc.entities(false).count(), 2);
    assert_eq!(doc.entities(true).count(), 4);

    for e in doc.entities(false) {
        assert_eq!(e.properties().len(), 1);
        assert_eq!(e.children.as_ref().unwrap().len(), 1);
    }
}

/// Insert three nested documents in row
#[test]
fn insert_docs_serial() {
    let mut doc = Document::new(333);
    doc.create_entity().add(COLOR, 3);
    assert!(doc.commit_transaction().is_ok());

    assert!(doc.switch(222).is_ok());
    doc.create_entity().add(INS_DOC, 333 as DocId);
    assert!(doc.commit_transaction().is_ok());

    assert!(doc.switch(111).is_ok());
    doc.create_entity().add(COLOR, 1);
    doc.create_entity().add(INS_DOC, 222 as DocId);
    assert!(doc.commit_transaction().is_ok());

    assert!(doc
        .get_entity(vec![START_NAME + 1, START_NAME, START_NAME])
        .is_some());

    assert_eq!(doc.entities(true).count(), 4);
    assert_eq!(doc.entities(false).count(), 2);
}

// Get entity by full name
#[test]
fn get_entity() {
    let mut doc = Document::new(222);
    doc.create_entity().add(COLOR, 3);
    assert!(doc.commit_transaction().is_ok());

    assert!(doc.switch(111).is_ok());
    doc.create_entity().add(COLOR, 1);
    doc.create_entity().add(INS_DOC, 222 as DocId);
    assert!(doc.commit_transaction().is_ok());

    assert!(doc.get_entity(vec![START_NAME]).is_some());
    assert!(doc.get_entity(vec![START_NAME + 1]).is_some());
    assert!(doc.get_entity(vec![START_NAME + 2]).is_none());
    assert!(doc.get_entity(vec![START_NAME + 1, START_NAME]).is_some());
    assert!(doc
        .get_entity(vec![START_NAME + 1, START_NAME + 1])
        .is_none());
}

#[test]
fn entity_iterator() {
    let mut doc = Document::new(333);
    assert!(doc.entities(true).next().is_none());
    assert!(doc.entities(false).next().is_none());

    assert!(doc.switch(222).is_ok());
    doc.create_entity().add(INS_DOC, 333 as DocId);
    assert!(doc.commit_transaction().is_ok());

    assert!(doc.switch(111).is_ok());
    doc.create_entity().add(INS_DOC, 222 as DocId);
    assert!(doc.commit_transaction().is_ok());

    let mut it = doc.entities(true);
    assert!(it.next().is_some());
    assert!(it.next().is_some());
    assert!(it.next().is_none());
    assert!(it.next().is_none());
}

#[test]
fn change_inserted_entity() {
    let mut doc = Document::new(222);
    doc.create_entity().add(COLOR, 22);
    assert!(doc.commit_transaction().is_ok());

    assert!(doc.switch(111).is_ok());
    doc.create_entity().add(COLOR, 0);
    doc.create_entity().add(INS_DOC, 222 as DocId);
    assert!(doc.commit_transaction().is_ok());

    doc.update_entity(vec![START_NAME + 1, START_NAME])
        .add(COLOR, 11);
    assert!(doc.commit_transaction().is_ok());

    // Ensure that only the entity in inserted document changed; the original document remain unchanged
    assert_eq!(
        doc.get_property::<i32>(vec![START_NAME + 1, START_NAME], COLOR)
            .unwrap(),
        11
    );
    assert!(doc.switch(222).is_ok());
    assert_eq!(
        doc.get_property::<i32>(vec![START_NAME], COLOR).unwrap(),
        22
    );
}

#[test]
fn delete_inserted_entity() {
    let mut doc = Document::new(222);
    doc.create_entity().add(COLOR, 22);
    assert!(doc.commit_transaction().is_ok());

    assert!(doc.switch(111).is_ok());
    doc.create_entity().add(INS_DOC, 222 as DocId);
    assert!(doc.commit_transaction().is_ok());

    doc.delete_entity(vec![START_NAME, START_NAME]);
    assert!(doc.commit_transaction().is_ok());

    // Entity deleted only in the active document, original one remain unchanged
    assert_eq!(doc.entities(true).count(), 0);
    assert!(doc.switch(222).is_ok());
    assert_eq!(doc.entities(true).count(), 1);
}

#[test]
fn clipboard() {
    let mut doc = Document::new(222);
    doc.create_entity().add(COLOR, 11);
    doc.create_entity().add(COLOR, 22);
    assert!(doc.commit_transaction().is_ok());

    assert!(doc.switch(111).is_ok());
    doc.create_entity().add(INS_DOC, 222 as DocId);
    doc.create_entity().add(COLOR, 33);
    assert!(doc.commit_transaction().is_ok());
    assert_eq!(doc.entities(true).count(), 4);

    let selected = BTreeSet::from([vec![START_NAME + 1], vec![START_NAME, START_NAME]]);
    let clipboard = doc.copy(selected);
    doc.paste(clipboard);
    assert!(doc.commit_transaction().is_ok());
    assert_eq!(doc.entities(true).count(), 6);
}
