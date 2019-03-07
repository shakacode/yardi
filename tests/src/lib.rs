use injector::*;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::default::Default;

#[derive(Clone)]
pub struct MyTest0 {
    test1: Arc<MyTest1>,
    test2: Arc<MyTest2>,
}

impl MyTest0 {
    pub fn new(test1: Arc<MyTest1>, test2: Arc<MyTest2>) -> Self {
        Self { test1, test2 }
    }

    fn test(&self) -> usize {
        self.test2.test()
    }
}

#[derive(Clone, Default)]
pub struct Orphan(usize);

dependencies! {
    consts {
        TEST: usize = 10
    },

    services {
        TestX {
            struct=MyTest0,
            args=[Test1, Test2],
        },

        Test1 {
            struct=Arc<MyTest1>,
            ctor=|| MyTest1::xxxx().into(),
        },

        Test2 {
            struct=Arc<MyTest2>,
            ctor=|a, c| MyTest2::new(a, c).into(),
            args=[Test1, TEST],
        },

        OrphanDep {
            struct = Orphan,
            ctor=default,
        }
    },
}

pub struct MyTest1 {
    x: AtomicUsize
}

impl Clone for MyTest1 {
    fn clone(&self) -> Self {
        MyTest1 {
            x: AtomicUsize::new(self.x.load(Ordering::SeqCst))
        }
    }
}

impl MyTest1 {
    pub fn xxxx() -> Self  {
        MyTest1 {
            x: AtomicUsize::new(1)
        }
    }

    pub fn inc(&self) {
        self.x.fetch_add(1, Ordering::SeqCst);
    } 

    pub fn get(&self) -> usize {
        self.x.load(Ordering::SeqCst)
    } 
}

#[derive(Clone)]
pub struct MyTest2 {
    test1: Arc<MyTest1>,
    test: usize,
}

impl MyTest2 {
    pub fn new(test1: Arc<MyTest1>, test: usize) -> Self  {
        MyTest2 { test1, test }
    }

    pub fn test(&self) -> usize {
        self.test1.inc();
        self.test + self.test1.get()
    } 
}

#[test]
fn test_dependencies() {
    let inj = Injector::new();
    let dep = inject!(inj, deps::TestX);
    assert_eq!(12, dep.test());
}

#[test]
fn test_send_sync() {
    let inj = std::sync::Arc::new(Injector::new());
    let clone = inj.clone();

    let dep = inject!(inj, deps::TestX);
    assert_eq!(12, dep.test());

    std::thread::spawn(move || {
        let dep = inject!(clone, deps::TestX);
        assert_eq!(13, dep.test());
    });
}