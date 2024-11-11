#![forbid(unsafe_code)]

pub use gc_derive::Scan;

use std::{
    cell::RefCell,
    collections::HashSet,
    marker::PhantomData,
    ops::Deref,
    rc::{Rc, Weak},
};

////////////////////////////////////////////////////////////////////////////////

pub struct Gc<T> {
    weak: Weak<T>,
}

impl<T> Clone for Gc<T> {
    fn clone(&self) -> Self {
        Self {
            weak: self.weak.clone(),
        }
    }
}

impl<T> Gc<T> {
    pub fn borrow(&self) -> GcRef<'_, T> {
        GcRef {
            rc: self.weak.upgrade().unwrap(),
            lifetime: PhantomData,
        }
    }
}

pub struct GcRef<'a, T> {
    rc: Rc<T>,
    lifetime: PhantomData<&'a Gc<T>>,
}

impl<'a, T> Deref for GcRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.rc
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait Scan {
    fn collect_gcs(&self) -> Vec<usize>;
}

impl Scan for i32 {
    fn collect_gcs(&self) -> Vec<usize> {
        vec![]
    }
}

impl<T> Scan for Gc<T> {
    fn collect_gcs(&self) -> Vec<usize> {
        vec![self.weak.as_ptr() as usize]
    }
}

impl<T: Scan> Scan for Option<T> {
    fn collect_gcs(&self) -> Vec<usize> {
        match self {
            Some(x) => x.collect_gcs(),
            None => vec![],
        }
    }
}

impl<T: Scan> Scan for Vec<T> {
    fn collect_gcs(&self) -> Vec<usize> {
        self.iter().flat_map(Scan::collect_gcs).collect()
    }
}

impl<T: Scan> Scan for RefCell<T> {
    fn collect_gcs(&self) -> Vec<usize> {
        self.borrow().collect_gcs()
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct Arena {
    allocations: Vec<Rc<dyn Scan + 'static>>,
}

impl Arena {
    pub fn new() -> Self {
        Self {
            allocations: Vec::new(),
        }
    }

    pub fn allocation_count(&self) -> usize {
        self.allocations.len()
    }

    pub fn alloc<T: Scan + 'static>(&mut self, object: T) -> Gc<T> {
        let allocation = Rc::new(object);
        let gc = Gc {
            weak: Rc::downgrade(&allocation),
        };

        self.allocations.push(allocation);

        gc
    }

    pub fn sweep(&mut self) {
        let mut internal_reference_counts = vec![0; self.allocation_count()];
        self.allocations.iter().for_each(|allocation| {
            allocation.collect_gcs().iter().for_each(|address| {
                if let Some(index) = self.find_index_by_address(*address) {
                    internal_reference_counts[index] += 1;
                }
            })
        });

        let mut marked = HashSet::<usize>::new();
        self.allocations
            .iter()
            .enumerate()
            .for_each(|(i, allocation)| {
                if Rc::weak_count(allocation) > internal_reference_counts[i] {
                    self.mark_all(Rc::as_ptr(allocation) as *const () as usize, &mut marked);
                }
            });

        self.allocations
            .retain(|allocation| marked.contains(&(Rc::as_ptr(allocation) as *const () as usize)));
    }

    fn find_index_by_address(&self, address: usize) -> Option<usize> {
        self.allocations
            .iter()
            .position(|allocation| Rc::as_ptr(allocation) as *const () as usize == address)
    }

    fn mark_all(&self, root_address: usize, marked: &mut HashSet<usize>) {
        if !marked.insert(root_address) {
            return;
        }

        if let Some(index) = self.find_index_by_address(root_address) {
            self.allocations[index]
                .collect_gcs()
                .iter()
                .for_each(|&address| self.mark_all(address, marked));
        }
    }
}
