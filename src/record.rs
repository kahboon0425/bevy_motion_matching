use std::collections::VecDeque;
use std::fmt::Debug;
use std::marker::PhantomData;

use bevy::prelude::*;

#[derive(Default)]
pub struct RecordPlugin<T: Recordable>(PhantomData<T>);

impl<T: Recordable> Plugin for RecordPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                record_len::<T>,
                record::<T>,
                // draw_transform2d_record_axes,
            )
                .chain(),
        );
    }
}

/// Push in a new [`Transform2dComp`] to the front of a [`Transform2dRecord`] while popping out an old one.
fn record<T: Recordable>(mut q_records: Query<(&T, &mut Records<T>)>, time: Res<Time>) {
    for (&value, mut record) in q_records.iter_mut() {
        record.pop_back();
        record.push_front(Record {
            value,
            delta_time: time.delta_seconds(),
        });
    }
}

/// Update size of [`Record`] if there are changes to [`RecordLen`].
fn record_len<T: Recordable>(
    mut q_records: Query<(&RecordLen<T>, &mut Records<T>), Changed<RecordLen<T>>>,
) {
    for (len, mut records) in q_records.iter_mut() {
        let target_len = **len;
        if records.len() != target_len {
            **records = VecDeque::from_iter(vec![default(); target_len]);
        }
    }
}

#[derive(Bundle)]
pub struct RecordsBundle<T: Recordable> {
    pub records: Records<T>,
    pub len: RecordLen<T>,
}

impl<T: Recordable> RecordsBundle<T> {
    pub fn new(len: usize) -> Self {
        Self {
            records: Records::default(),
            len: RecordLen::new(len),
        }
    }
}

/// A history record of the target [`Transform2dComp`] component.
#[derive(Component, Default, Debug, Deref, DerefMut, Clone)]
pub struct Records<T: Recordable>(VecDeque<Record<T>>);

#[derive(Default, Debug, Clone, Copy)]
pub struct Record<T: Recordable> {
    /// The recorded value.
    pub value: T,
    /// The time between the previous frame and the frame where the transform is recorded.
    pub delta_time: f32,
}

/// Determines the size of [`Transform2dRecord`].
#[derive(Component, Default, Debug, Deref, DerefMut, Clone, Copy)]
pub struct RecordLen<T: Recordable>(#[deref] usize, PhantomData<T>);

impl<T: Recordable> RecordLen<T> {
    fn new(len: usize) -> Self {
        Self(len, PhantomData)
    }
}

pub trait Recordable: Component + Default + Debug + Clone + Copy {}

impl<T> Recordable for T where T: Component + Default + Debug + Clone + Copy {}
