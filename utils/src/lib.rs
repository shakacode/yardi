pub type InjectorError = Box<dyn std::error::Error + 'static>;

pub trait Dep {
    type DependecyType;
}

pub trait Inject<T: Dep> {
    fn inject(&self) -> T::DependecyType;
}
