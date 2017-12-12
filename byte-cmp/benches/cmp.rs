extern crate rand;
#[macro_use]
extern crate bencher;
extern crate libc;

use libc::{c_void, memcmp};

use bencher::Bencher;
use rand::Rng;

type T = [u8; 32];

fn eq_loop(b: &mut Bencher) {
    let mut rng = rand::thread_rng(); 
    let mut t1: T = Default::default();
    let mut t2: T = Default::default();
    rng.fill_bytes(&mut t1);
    rng.fill_bytes(&mut t2);
    b.iter(|| 
        for _ in 0..1_000 {
            assert!(!(t1 == t2))
        }
    );
}

fn cmp_loop(b: &mut Bencher) {
    let mut rng = rand::thread_rng(); 
    let mut t1: T = Default::default();
    let mut t2: T = Default::default();
    rng.fill_bytes(&mut t1);
    rng.fill_bytes(&mut t2);
    let t1ptr = t1.as_ptr() as *const c_void;
    let t2ptr = t2.as_ptr() as *const c_void;
    b.iter(|| 
           for _ in 0..1_000 {
               assert!(!(unsafe { memcmp(t1ptr, t2ptr, 32) == 0 }));
           }
           );
}

fn eq(b: &mut Bencher) {
    let mut rng = rand::thread_rng(); 
    let mut t1: T = Default::default();
    let mut t2: T = Default::default();
    rng.fill_bytes(&mut t1);
    rng.fill_bytes(&mut t2);
    b.iter(|| 
            assert!(!(t1 == t2))
    );
}

fn cmp(b: &mut Bencher) {
    let mut rng = rand::thread_rng(); 
    let mut t1: T = Default::default();
    let mut t2: T = Default::default();
    rng.fill_bytes(&mut t1);
    rng.fill_bytes(&mut t2);
    let t1ptr = t1.as_ptr() as *const c_void;
    let t2ptr = t2.as_ptr() as *const c_void;
    b.iter(|| 
               assert!(!(unsafe { memcmp(t1ptr, t2ptr, 32) == 0 }))
           );
}


benchmark_group!(loops, eq_loop, cmp_loop);
benchmark_group!(comparisons, eq, cmp);

benchmark_main!(comparisons, loops);
