use ecs::{ComponentAccess, ResourceAccess};

const EMPTY: &[usize] = &[];

#[test]
fn component_access_detects_conflicting_writes() {
    static READ: [usize; 1] = [1];
    static WRITE: [usize; 1] = [2];
    static OTHER_WRITE: [usize; 1] = [2];

    let a = ComponentAccess {
        read: &READ,
        write: EMPTY,
    };
    let b = ComponentAccess {
        read: EMPTY,
        write: &OTHER_WRITE,
    };
    let c = ComponentAccess {
        read: EMPTY,
        write: &WRITE,
    };

    assert!(b.overlaps(&c));
    assert!(c.overlaps(&b));
    assert!(!a.overlaps(&b));
}

#[test]
fn component_access_detects_read_write_conflicts() {
    static READ: [usize; 1] = [3];
    static WRITE: [usize; 1] = [3];

    let reader = ComponentAccess {
        read: &READ,
        write: EMPTY,
    };
    let writer = ComponentAccess {
        read: EMPTY,
        write: &WRITE,
    };

    assert!(reader.overlaps(&writer));
    assert!(writer.overlaps(&reader));
}

#[test]
fn resource_access_mirrors_component_access_rules() {
    static READ: [usize; 1] = [7];
    static WRITE: [usize; 1] = [7];

    let read_only = ResourceAccess {
        read: &READ,
        write: EMPTY,
    };
    let write_only = ResourceAccess {
        read: EMPTY,
        write: &WRITE,
    };
    let disjoint_write = ResourceAccess {
        read: EMPTY,
        write: &[9],
    };

    assert!(read_only.overlaps(&write_only));
    assert!(write_only.overlaps(&read_only));
    assert!(!write_only.overlaps(&disjoint_write));
}
