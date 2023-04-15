use prost_types::FileDescriptorSet;

pub trait Preprocessor {
    fn process(&mut self, fds: &mut FileDescriptorSet, buf: &mut String);
}
