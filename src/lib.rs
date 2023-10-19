mod entity;
mod property;
mod transaction;

#[cfg(test)]
mod tests {

    use crate::entity::*;
    use crate::property::*;

    #[test]
    fn create() {
        let mut doc = Document::new(1);

        let ce = doc.trs().create_entity();
        ce.add(Value::Color(101));
        ce.add(Value::Title(String::from("qwerty")));
        doc.trs().close();
        assert!(doc.apply_transaction().is_ok());

        assert_eq!(doc.entities().count(), 1);

        let e = doc.entities().next().unwrap();
        assert_eq!(e.properties().count(), 2);
        assert_eq!(*e.get_property(COLOR).unwrap(), Value::Color(101));
        assert_eq!(
            *e.get_property(TITLE).unwrap(),
            Value::Title(String::from("qwerty"))
        );
    }

    #[test]
    fn update() {
        let mut doc = Document::new(1);

        let ce = doc.trs().create_entity();
        ce.add(Value::Color(101));
        doc.trs().close();
        assert!(doc.apply_transaction().is_ok());
        assert_eq!(doc.entities().count(), 1);

        let ue = doc.trs().update_entity(vec![START_NAME]);
        ue.add(Value::Color(11));
        doc.trs().close();
        assert!(doc.apply_transaction().is_ok());

        let e = doc.entities().next().unwrap();
        assert_eq!(e.properties().count(), 1);
        assert_eq!(*e.get_property(COLOR).unwrap(), Value::Color(11));
    }

    #[test]
    fn create_two() {
        let mut doc = Document::new(1);

        let ce = doc.trs().create_entity();
        ce.add(Value::Color(101));
        ce.add(Value::Title(String::from("qwerty")));
        assert!(doc.apply_transaction().is_ok());

        let ce = doc.trs().create_entity();
        ce.add(Value::Title(String::from("qwerty")));
        doc.trs().close();
        assert!(doc.apply_transaction().is_ok());

        assert_eq!(doc.entities().count(), 2);
    }

    #[test]
    fn delete_entity() {
        let mut doc = Document::new(1);

        let ce = doc.trs().create_entity();
        ce.add(Value::Color(101));
        doc.trs().close();
        assert!(doc.apply_transaction().is_ok());
        assert_eq!(doc.entities().count(), 1);

        doc.trs().delete_entity(vec![START_NAME]);
        doc.trs().close();
        assert!(doc.apply_transaction().is_ok());
        assert_eq!(doc.entities().count(), 0);
    }

    #[test]
    fn delete_property() {
        let mut doc = Document::new(1);

        let ce = doc.trs().create_entity();
        ce.add(Value::Color(101));
        let name = ce.ename.clone();
        doc.trs().close();
        assert!(doc.apply_transaction().is_ok());
        assert!(!doc.get_entity(&name).unwrap().props.is_empty());

        let ce = doc.trs().update_entity(name.clone());
        ce.delete(COLOR);
        doc.trs().close();
        assert!(doc.apply_transaction().is_ok());
        assert!(doc.get_entity(&name).unwrap().props.is_empty());
    }

    #[test]
    fn undo_redo() {
        let mut doc = Document::new(1);
        assert_eq!(doc.history_size(), (0, 0));
        assert!(doc.undo(-1).is_ok() == false);

        {
            let ce = doc.trs().create_entity();
            ce.add(Value::Color(101));
            doc.trs().close();
            assert!(doc.apply_transaction().is_ok());
        }
        assert_eq!(doc.history_size(), (1, 1));

        {
            let ce = doc.trs().update_entity(vec![START_NAME]);
            ce.add(Value::Color(102));
            doc.trs().close();
            assert!(doc.apply_transaction().is_ok());
        }
        assert_eq!(doc.history_size(), (2, 2));

        assert!(doc.undo(-1).is_ok());
        assert_eq!(
            *doc.get_entity(&vec![START_NAME])
                .unwrap()
                .get_property(COLOR)
                .unwrap(),
            Value::Color(101)
        );
        assert_eq!(doc.history_size(), (2, 1));

        assert!(doc.undo(1).is_ok());
        assert_eq!(
            *doc.get_entity(&vec![START_NAME])
                .unwrap()
                .get_property(COLOR)
                .unwrap(),
            Value::Color(102)
        );
        assert_eq!(doc.history_size(), (2, 2));
        assert!(doc.undo(1).is_ok() == false);
    }

    #[test]
    fn name_is_unique() {
        let mut doc = Document::new(1);

        doc.trs().create_entity();
        doc.trs().close();
        assert!(doc.apply_transaction().is_ok());
        let name_0 = doc.entities().next().unwrap().name.clone();

        doc.trs().delete_entity(name_0.clone());
        doc.trs().close();
        doc.apply_transaction().ok();

        doc.trs().create_entity();
        doc.trs().close();
        assert!(doc.apply_transaction().is_ok());
        assert_eq!(doc.entities().count(), 1);
        // name of the removed entity is reserved and will never be used again
        assert!(name_0 != doc.entities().next().unwrap().name);
    }

    #[test]
    fn trs_len_grow() {
        let mut doc = Document::new(1);

        let l0 = doc.trs().len();
        let ce = doc.trs().create_entity();
        ce.add(Value::Color(101));
        let l1 = doc.trs().len();
        assert!(l0 < l1);

        let ce2 = doc.trs().create_entity();
        ce2.add(Value::Color(101));
        ce2.add(Value::Title(String::from("qwerty")));
        let l2 = doc.trs().len();
        assert!(l1 < l2);
        assert!(l1 - l0 < l2 - l1);

        doc.trs().close();
        assert_eq!(doc.trs().len(), l2);
    }
}
