use std::ops::Deref;

use stack_vector::StackVec;

#[test]
fn push() {
    let mut sv = StackVec::<_, 10>::new();
    assert_eq!(sv.capacity(), 10);
    assert_eq!(sv.remaining_capacity(), 10);

    assert!(sv.is_empty());
    assert!(!sv.is_full());
    assert_eq!(sv.len(), 0);

    for i in 1..=10 {
        assert!(sv.try_push(i).is_ok());
        assert_eq!(sv.remaining_capacity(), (10 - i) as usize);
    }

    assert!(!sv.is_empty());
    assert!(sv.is_full());
    assert_eq!(sv.len(), 10);
    assert_eq!(sv.remaining_capacity(), 0);
    assert_eq!(sv.try_push(-1), Err(-1));

    assert_eq!(sv.as_slice(), &[1,2,3,4,5,6,7,8,9,10]);
}

#[test]
#[should_panic(expected = "Attemp to push beyond the capacity of the array")]
fn out_of_bounds_must_panic() {
    let mut sv = StackVec::<_, 10>::new();
    for i in 0..10 {
        sv.push(i)
    }
    sv.push(-1);
}

#[test]
fn remove() {
    let mut sv = StackVec::from_array([1, 2, 3, 4, 5, 6]);

    assert_eq!(sv.remove(1), Some(2));
    assert_eq!(sv.remove(4), Some(6));
    assert_eq!(sv.remove(0), Some(1));

    assert_eq!(sv.deref(), &[3, 4, 5]);
}

#[test]
fn constructors() {
    let mut i = 0;
    let mut sv = StackVec::<i32, 10>::generate(|| {
        i += 1;
        i
    });

    sv.remove(5);
    sv[1] = -1;

    assert_eq!(sv.as_slice(), &[1, -1, 3, 4, 5, 7, 8, 9, 10]);

    let sv = StackVec::<i32, 5>::filled(0);
    assert_eq!(sv.as_slice(), &[0, 0, 0, 0, 0]);
}

#[test]
fn drain() {
    let mut sv = StackVec::<i32, 10>::from_array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    let d = sv.drain(1..=3).collect::<Vec<_>>();
    assert_eq!(d, [1, 2, 3]);

    assert_eq!(sv.as_slice(), &[0, 4, 5, 6, 7, 8, 9]);

    assert_eq!(sv.len(), 7);
}
