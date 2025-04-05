#![feature(test)]
extern crate test;
use stack_vector::StackVec;
use test::Bencher;

const N: usize = 99999;

#[bench]
fn bench_stack_vec(b: &mut Bencher) {
    b.iter(|| {
        let mut v = StackVec::<_, N>::new();
        for i in 0..N {
            v.push(i);
        }
    })
}

#[bench]
fn bench_vector(b: &mut Bencher) {
    b.iter(|| {
        let mut v = Vec::with_capacity(N);
        for i in 0..N {
            v.push(i);
        }
    })
}
