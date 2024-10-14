use std::{
    num::NonZero,
    ops::{Index, IndexMut},
};

use crate::game::PlayerId;

pub struct PlayerIndexedVector<T> {
    data: Vec<T>,
}

impl<T> From<Vec<T>> for PlayerIndexedVector<T> {
    fn from(value: Vec<T>) -> Self {
        Self { data: value }
    }
}

impl<T: Default> PlayerIndexedVector<T> {
    pub fn new(player_amount: usize) -> Self {
        Self {
            data: std::iter::repeat_with(T::default)
                .take(player_amount)
                .collect(),
        }
    }
}

impl<T> PlayerIndexedVector<T> {
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn into_vec(self) -> Vec<T> {
        self.data
    }

    pub fn iter(&self) -> impl Iterator<Item = (PlayerId, &T)> {
        self.data
            .iter()
            .enumerate()
            .map(|(i, p)| (NonZero::new(i + 1).unwrap(), p))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (PlayerId, &mut T)> {
        self.data
            .iter_mut()
            .enumerate()
            .map(|(i, p)| (NonZero::new(i + 1).unwrap(), p))
    }

    pub fn iter_player_ids(&self) -> impl Iterator<Item = PlayerId> {
        (0..self.len()).map(|i| PlayerId::new(i + 1).unwrap())
    }

    pub fn map<E, F: FnMut(&T) -> E>(&self, f: F) -> PlayerIndexedVector<E> {
        PlayerIndexedVector::<E>::from(self.data.iter().map(f).collect::<Vec<_>>())
    }

    pub fn mapped<E, F: FnMut(T) -> E>(self, f: F) -> PlayerIndexedVector<E> {
        PlayerIndexedVector::<E>::from(self.data.into_iter().map(f).collect::<Vec<_>>())
    }
}

impl<T> Index<PlayerId> for PlayerIndexedVector<T> {
    type Output = T;

    fn index(&self, index: PlayerId) -> &Self::Output {
        &self.data[index.get() - 1]
    }
}

impl<T> IndexMut<PlayerId> for PlayerIndexedVector<T> {
    fn index_mut(&mut self, index: PlayerId) -> &mut Self::Output {
        &mut self.data[index.get() - 1]
    }
}

impl<T> FromIterator<T> for PlayerIndexedVector<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            data: iter.into_iter().collect(),
        }
    }
}
