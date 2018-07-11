#![deny(warnings)]
use std::any::Any;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::hash_map::OccupiedEntry;
use std::collections::hash_map::Entry::{ Occupied, Vacant };
use std::fmt;
use std::fmt::{ Debug, Formatter };
use std::hash::{ Hash, Hasher };
use std::marker::{ PhantomData, Send };
use std::mem::replace;
use std::ops::{ Deref, DerefMut };
use std::sync::{ Arc, Mutex, MutexGuard };
use either::{ Either, Left, Right };

pub trait ValTypeDesc<I> : Send {
    fn name(&self) -> &str;
    fn parse(&self, type_: ValType<I>, s: &str) -> Option<Arc<Val<I>>>;
    fn to_string(&self, val: &Val<I>) -> String;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
struct ValTypeI {
    index: usize
}

pub struct ValType<I>(ValTypeI, PhantomData<I>);

impl<I> Debug for ValType<I> { fn fmt(&self, f: &mut Formatter) -> fmt::Result { self.0.fmt(f) } }
impl<I> Copy for ValType<I> { }
impl<I> Clone for ValType<I> { fn clone(&self) -> Self { ValType(self.0, PhantomData) } }
impl<I> PartialEq for ValType<I> { fn eq(&self, other: &ValType<I>) -> bool { self.0 == other.0 } }
impl<I> Eq for ValType<I> { }
impl<I> Ord for ValType<I> { fn cmp(&self, other: &ValType<I>) -> Ordering { self.0.cmp(&other.0) } }
impl<I> PartialOrd for ValType<I> { fn partial_cmp(&self, other: &ValType<I>) -> Option<Ordering> { Some(self.cmp(other)) } }
impl<I> Hash for ValType<I> { fn hash<H: Hasher>(&self, state: &mut H) { self.0.hash(state); } }

impl<I> ValType<I> {
    pub fn box_<T: 'static + Any + Send + Sync>(&self, val: T) -> Arc<Val<I>> { Arc::new(Val(ValI { type_: self.0, unbox: Box::new(val) }, PhantomData)) }
    pub fn name(self, fw: &Fw<I>) -> &str {
        fw.val_types[self.0.index].name()
    }
    pub fn parse(self, s: &str, fw: &Fw<I>) -> Option<Arc<Val<I>>> {
        fw.val_types[self.0.index].parse(self, s)
    }
}

struct ValI {
    type_: ValTypeI,
    unbox: Box<Any + Send + Sync>,
}

pub struct Val<I>(ValI, PhantomData<I>);

impl<I> Val<I> {
    pub fn type_(&self) -> ValType<I> { ValType(self.0.type_, PhantomData) }
    pub fn unbox<T: 'static>(&self) -> &T { <Any>::downcast_ref(&*self.0.unbox).unwrap() }
    pub fn to_string(&self, fw: &Fw<I>) -> String {
        fw.val_types[self.0.type_.index].to_string(self)
    }
}

struct DepPropClass<I> {
    def_val: Option<Obj<I>>,
    set_lock: Option<ClassSetLock>,
    on_changed: Vec<Box<Fn(&Arc<DepObj<I>>, &Obj<I>, &Obj<I>, &Fw<I>) + Send>>,
}

struct DepTypeDesc<I> {
    base: Option<DepType<I>>,
    name: String,
    props: Vec<DepPropDesc<I>>,
    props_by_name: HashMap<String, DepProp<I>>,
    prop_class: HashMap<DepProp<I>, DepPropClass<I>>,
    ctor: Option<Box<Fn(&Arc<DepObj<I>>, &Fw<I>) + Send>>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct DepTypeI {
    index: usize
}

pub struct DepType<I>(DepTypeI, PhantomData<I>);

impl<I> Debug for DepType<I> { fn fmt(&self, f: &mut Formatter) -> fmt::Result { self.0.fmt(f) } }
impl<I> Copy for DepType<I> { }
impl<I> Clone for DepType<I> { fn clone(&self) -> Self { DepType(self.0, PhantomData) } }
impl<I> PartialEq for DepType<I> { fn eq(&self, other: &DepType<I>) -> bool { self.0 == other.0 } }
impl<I> Eq for DepType<I> { }
impl<I> Ord for DepType<I> { fn cmp(&self, other: &DepType<I>) -> Ordering { self.0.cmp(&other.0) } }
impl<I> PartialOrd for DepType<I> { fn partial_cmp(&self, other: &DepType<I>) -> Option<Ordering> { Some(self.cmp(other)) } }
impl<I> Hash for DepType<I> { fn hash<H: Hasher>(&self, state: &mut H) { self.0.hash(state); } }

fn assert_dep_prop_target<I>(dep_prop: DepProp<I>, dep_type: DepType<I>, fw: &Fw<I>) {
    if !dep_type.is(dep_prop.target(fw), fw) {
        panic!("Dependency property target type mismatch.");
    }
}

fn assert_dep_prop_val<I>(dep_prop: DepProp<I>, val_type: Type<I>, fw: &Fw<I>) {
    if !val_type.is(dep_prop.val_type(fw), fw) {
        panic!("Dependency property value type mismatch.");
    }
}

impl<I> DepType<I> {
    pub fn name(self, fw: &Fw<I>) -> &str {
        &fw.dep_types[self.0.index].name
    }
    pub fn base(self, fw: &Fw<I>) -> Option<DepType<I>> {
        fw.dep_types[self.0.index].base
    }
    pub fn def_val(self, dep_prop: DepProp<I>, fw: &Fw<I>) -> &Obj<I> {
        assert_dep_prop_target(dep_prop, self, fw);
        fn self_def_val<I>(dep_type: DepType<I>, fw: &Fw<I>, dep_prop: DepProp<I>) -> Option<&Obj<I>> {
            fw.dep_types[dep_type.0.index].prop_class.get(&dep_prop).and_then(|c| { c.def_val.as_ref() })
        }
        let mut base = self;
        loop {
            if let Some(ref val) = self_def_val(base, fw, dep_prop) {
                return val;
            }
            if let Some(t) = base.base(fw) { base = t; } else { panic!("DEF_VAL_NOT_FOUND"); }
        }
    }
    fn init(self, obj: &Arc<DepObj<I>>, fw: &Fw<I>) {
        if let Some(base) = self.base(fw) {
            base.init(obj, fw);
        }
        if let Some(ref ctor) = fw.dep_types[self.0.index].ctor {
            ctor(obj, fw);
        }
    }
    pub fn create(self, fw: &Fw<I>) -> Arc<DepObj<I>> {
        let obj = Arc::new(DepObj { type_: self, props: Mutex::new(HashMap::new()), data: Mutex::new(HashMap::new()) });
        self.init(&obj, fw);
        obj
    }
    pub fn is(self, dep_type: DepType<I>, fw: &Fw<I>) -> bool {
        let mut base = self;
        loop {
            if base == dep_type { return true; }
            if let Some(t) = base.base(fw) { base = t; } else { return false; }
        }
    }
    fn set_lock(self, dep_prop: DepProp<I>, fw: &Fw<I>) -> Option<ClassSetLock> {
        let mut base = self;
        loop {
            let maybe_lock = fw.dep_types.get(base.0.index).unwrap().prop_class.get(&dep_prop)
                .and_then(|class| { class.set_lock.clone() });
            if maybe_lock.is_some() { return maybe_lock; }
            if let Some(t) = base.base(fw) { base = t; } else { return None; }
        }
    }
    pub fn is_locked(self, dep_prop: DepProp<I>, fw: &Fw<I>) -> bool {
        self.set_lock(dep_prop, fw).is_some()
    }
}

pub enum Type<I> {
    Val(ValType<I>),
    Dep(DepType<I>),
    Opt(Box<Type<I>>),
}

impl<I> Debug for Type<I> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Type::Val(v) => write!(f, "Val({:?})", v),
            Type::Dep(d) => write!(f, "Dep({:?})", d),
            Type::Opt(t) => write!(f, "Opt({:?})", t),
        }
    }
}
impl<I> Clone for Type<I> {
    fn clone(&self) -> Self {
        match self {
            Type::Val(v) => Type::Val(v.clone()),
            Type::Dep(d) => Type::Dep(d.clone()),
            Type::Opt(t) => Type::Opt(t.clone()),
        }
    }
}
impl<I> PartialEq for Type<I> {
    fn eq(&self, other: &Type<I>) -> bool {
        match self {
            Type::Val(v) => { if let Type::Val(o_v) = other { *v == *o_v } else { false } },
            Type::Dep(d) => { if let Type::Dep(o_d) = other { *d == *o_d } else { false } },
            Type::Opt(t) => { if let Type::Opt(o_t) = other { *t == *o_t } else { false } },
        }
    }
}
impl<I> Eq for Type<I> { }
impl<I> Ord for Type<I> {
    fn cmp(&self, other: &Type<I>) -> Ordering {
        match self {
            Type::Val(v) => {
                match other {
                    Type::Val(o_v) => v.cmp(o_v),
                    Type::Dep(_) => Ordering::Less,
                    Type::Opt(_) => Ordering::Less,
                }
            },
            Type::Dep(d) => {
                match other {
                    Type::Val(_) => Ordering::Greater,
                    Type::Dep(o_d) => d.cmp(o_d),
                    Type::Opt(_) => Ordering::Less,
                }
            },
            Type::Opt(t) => {
                match other {
                    Type::Val(_) => Ordering::Greater,
                    Type::Dep(_) => Ordering::Greater,
                    Type::Opt(o_t) => t.cmp(o_t),
                }
            },
        }
    }
}
impl<I> PartialOrd for Type<I> { fn partial_cmp(&self, other: &Type<I>) -> Option<Ordering> { Some(self.cmp(other)) } }
impl<I> Hash for Type<I> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Type::Val(v) => { state.write_u8(0); v.hash(state); },
            Type::Dep(d) => { state.write_u8(1); d.hash(state); },
            Type::Opt(t) => { state.write_u8(2); t.hash(state); },
        }
    }
}

impl<I> Type<I> {
    pub fn is(&self, type_: &Type<I>, fw: &Fw<I>) -> bool {
        match self {
            Type::Val(v) => { if let Type::Val(o_v) = type_ { v == o_v } else { false } },
            Type::Dep(d) => { if let Type::Dep(o_d) = type_ { d.is(*o_d, fw) } else { false } },
            Type::Opt(ref t) => { if let Type::Opt(ref o_t) = type_ { t.is(o_t, fw) } else { false } },
        }
    }
}

struct DepPropDesc<I> {
    name: String,
    val_type: Type<I>,
    attached: Option<DepType<I>>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct DepPropI {
    owner: DepTypeI,
    index: usize,
}

pub struct DepProp<I>(DepPropI, PhantomData<I>);

impl<I> Debug for DepProp<I> { fn fmt(&self, f: &mut Formatter) -> fmt::Result { self.0.fmt(f) } }
impl<I> Copy for DepProp<I> { }
impl<I> Clone for DepProp<I> { fn clone(&self) -> Self { DepProp(self.0, PhantomData) } }
impl<I> PartialEq for DepProp<I> { fn eq(&self, other: &DepProp<I>) -> bool { self.0 == other.0 } }
impl<I> Eq for DepProp<I> { }
impl<I> Ord for DepProp<I> { fn cmp(&self, other: &DepProp<I>) -> Ordering { self.0.cmp(&other.0) } }
impl<I> PartialOrd for DepProp<I> { fn partial_cmp(&self, other: &DepProp<I>) -> Option<Ordering> { Some(self.cmp(other)) } }
impl<I> Hash for DepProp<I> { fn hash<H: Hasher>(&self, state: &mut H) { self.0.hash(state); } }

impl<I> DepProp<I> {
    pub fn owner(self) -> DepType<I> { DepType(self.0.owner, PhantomData) }
    pub fn name(self, fw: &Fw<I>) -> &str {
        &fw.dep_types[self.0.owner.index].props[self.0.index].name[..]
    }
    pub fn val_type(self, fw: &Fw<I>) -> &Type<I> {
        &fw.dep_types[self.0.owner.index].props[self.0.index].val_type
    }
    pub fn attached(self, fw: &Fw<I>) -> Option<DepType<I>> {
        fw.dep_types[self.0.owner.index].props[self.0.index].attached
    }
    pub fn target(self, fw: &Fw<I>) -> DepType<I> {
        self.attached(fw).unwrap_or(DepType(self.0.owner, PhantomData))
    }
}

pub struct Fw<I> {
    val_types: Vec<Box<ValTypeDesc<I>>>,
    val_types_by_name: HashMap<String, ValType<I>>,
    dep_types: Vec<DepTypeDesc<I>>,
    dep_types_by_name: HashMap<String, DepType<I>>
}

#[derive(Debug, Clone)]
pub struct Unique(Arc<()>);

impl PartialEq for Unique { fn eq(&self, other: &Unique) -> bool { Arc::ptr_eq(&self.0, &other.0) } }
impl Eq for Unique { }
impl Ord for Unique { fn cmp(&self, other: &Unique) -> Ordering { (&*self.0 as *const ()).cmp(&(&*other.0 as *const ())) } }
impl PartialOrd for Unique { fn partial_cmp(&self, other: &Unique) -> Option<Ordering> { Some(self.cmp(other)) } }
impl Hash for Unique { fn hash<H: Hasher>(&self, state: &mut H) { (&*self.0 as *const ()).hash(state); } }

impl Unique {
    pub fn new() -> Unique { Unique(Arc::new(())) }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DepObjDataKey(Unique);

impl DepObjDataKey {
    pub fn new() -> DepObjDataKey { DepObjDataKey(Unique::new()) }
}

pub struct DepObj<I> {
    type_: DepType<I>,
    props: Mutex<HashMap<DepProp<I>, Obj<I>>>,
    data: Mutex<HashMap<DepObjDataKey, Box<Any + Send>>>,
}

pub struct GetRef<'a, I : 'static> {
    props: MutexGuard<'a, HashMap<DepProp<I>, Obj<I>>>,
    fw: &'a Fw<I>,
    dep_type: DepType<I>,
    dep_prop: DepProp<I>,
}

impl<'a, I> Deref for GetRef<'a, I> {
    type Target = Obj<I>;

    fn deref(&self) -> &Obj<I> {
        self.props.get(&self.dep_prop).unwrap_or_else(|| {
            self.dep_type.def_val(self.dep_prop, self.fw)
        })
    }
}

pub struct GetNonDefRef<'a, I : 'static> {
    props: MutexGuard<'a, HashMap<DepProp<I>, Obj<I>>>,
    dep_prop: DepProp<I>,
}

impl<'a, I> GetNonDefRef<'a, I> {
    pub fn borrow(&self) -> Option<&Obj<I>> {
        self.props.get(&self.dep_prop)
    }
}

pub struct GetDataRef<'a> {
    data: MutexGuard<'a, HashMap<DepObjDataKey, Box<Any + Send>>>,
    key: &'a DepObjDataKey,
}

impl<'a> GetDataRef<'a> {
    pub fn borrow(&self) -> Option<&Box<Any + Send>> {
        self.data.get(&self.key)
    }
}

impl<I> DepObj<I> {
    pub fn type_(&self) -> DepType<I> { self.type_ }
    pub fn get_non_def<'a>(&'a self, dep_prop: DepProp<I>, fw: &Fw<I>) -> GetNonDefRef<'a, I> {
        assert_dep_prop_target(dep_prop, self.type_, fw);
        GetNonDefRef { props: self.props.lock().unwrap(), dep_prop: dep_prop }
    }
    pub fn get<'a>(&'a self, dep_prop: DepProp<I>, fw: &'a Fw<I>) -> GetRef<'a, I> {
        assert_dep_prop_target(dep_prop, self.type_, fw);
        GetRef { props: self.props.lock().unwrap(), fw: fw, dep_prop: dep_prop, dep_type: self.type_ }
    }
    pub fn get_data<'a>(&'a self, key: &'a DepObjDataKey) -> GetDataRef<'a> {
        GetDataRef { data: self.data.lock().unwrap(), key: key }
    }
    pub fn set_data(&self, key: DepObjDataKey, value: Box<Any + Send>) {
        self.data.lock().unwrap().insert(key, value);
    }
    pub fn reset_data(&self, key: &DepObjDataKey) {
        self.data.lock().unwrap().remove(key);
    }
    pub fn set(&self, dep_prop: DepProp<I>, val: Obj<I>, fw: &Fw<I>) -> Result<(), ()> {
        self.set_core(dep_prop, val, None, fw)
    }
    pub fn set_locked(&self, dep_prop: DepProp<I>, val: Obj<I>, lock: &ClassSetLock, fw: &Fw<I>) {
        self.set_core(dep_prop, val, Some(lock), fw).unwrap();
    }
    fn set_core(&self, dep_prop: DepProp<I>, val: Obj<I>, lock: Option<&ClassSetLock>, fw: &Fw<I>) -> Result<(), ()> {
        assert_dep_prop_val(dep_prop, val.type_(), fw);
        self.check_set(dep_prop, lock, fw)?;
        self.props.lock().unwrap().insert(dep_prop, val);
        Ok(())
    }
    fn check_set(&self, dep_prop: DepProp<I>, lock: Option<&ClassSetLock>, fw: &Fw<I>) -> Result<(), ()> {
        assert_dep_prop_target(dep_prop, self.type_, fw);
        let set_lock = self.type_.set_lock(dep_prop, fw);
        if let Some(set_lock) = set_lock {
            if let Some(lock) = lock {
                if set_lock != *lock {
                    panic!("Invalid class lock.");
                }
            } else {
                return Err(());
            }
        } else if lock.is_some() {
            panic!("Invalid class lock.");
        }
        Ok(())
    }
    pub fn reset(&self, dep_prop: DepProp<I>, fw: &Fw<I>) -> Result<(), ()> {
        self.reset_core(dep_prop, None, fw)
    }
    pub fn reset_locked(&self, dep_prop: DepProp<I>, lock: &ClassSetLock, fw: &Fw<I>) -> Result<(), ()> {
        self.reset_core(dep_prop, Some(lock), fw)
    }
    fn reset_core(&self, dep_prop: DepProp<I>, lock: Option<&ClassSetLock>, fw: &Fw<I>) -> Result<(), ()> {
        self.check_set(dep_prop, lock, fw)?;
        self.props.lock().unwrap().remove(&dep_prop);
        Ok(())
    }
    pub fn is(&self, dep_type: DepType<I>, fw: &Fw<I>) -> bool {
        self.type_.is(dep_type, fw)
    }
}

pub enum Obj<I> {
    Val(Arc<Val<I>>),
    Dep(Arc<DepObj<I>>),
    Nil(Type<I>),
    Has(Arc<Obj<I>>),
}

impl<I> Clone for Obj<I> {
    fn clone(&self) -> Self {
        match self {
            Obj::Val(ref v) => Obj::Val(v.clone()),
            Obj::Dep(ref d) => Obj::Dep(d.clone()),
            Obj::Nil(ref t) => Obj::Nil(t.clone()),
            Obj::Has(ref o) => Obj::Has(o.clone()),
        }
    }
}

impl<I> Obj<I> {
    pub fn unbox<T: 'static>(&self) -> &T {
        if let Obj::Val(ref v) = self { v.unbox() } else { panic!("Cannot unbox a non-value object."); }
    }
    pub fn type_(&self) -> Type<I> {
        match self {
            Obj::Val(ref v) => Type::Val(v.type_()),
            Obj::Dep(ref d) => Type::Dep(d.type_()),
            Obj::Nil(ref t) => Type::Opt(Box::new(t.clone())),
            Obj::Has(ref o) => Type::Opt(Box::new(o.type_())),
        }
    }
    pub fn is(&self, type_: &Type<I>, fw: &Fw<I>) -> bool {
        self.type_().is(type_, fw)
    }
}

struct OccupiedDepPropClassRef<'a, I: 'static> {
    entry: OccupiedEntry<'a, DepProp<I>, DepPropClass<I>>,
}

impl<'a, I> Deref for OccupiedDepPropClassRef<'a, I> {
    type Target = DepPropClass<I>;

    fn deref(&self) -> &DepPropClass<I> {
        self.entry.get()
    }
}

impl<'a, I> DerefMut for OccupiedDepPropClassRef<'a, I> {
    fn deref_mut(&mut self) -> &mut DepPropClass<I> {
        self.entry.get_mut()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ClassSetLock(Unique);

impl<I: 'static> Fw<I> {
    pub fn new(_instance: I) -> Fw<I> {
        Fw { val_types: Vec::new(), val_types_by_name: HashMap::new(), dep_types: Vec::new(), dep_types_by_name: HashMap::new() }
    }
    pub fn val_type(&self, name: &str) -> Option<ValType<I>> {
        self.val_types_by_name.get(name).map(|x| { *x })
    }
    pub fn dep_type(&self, name: &str) -> Option<DepType<I>> {
        self.dep_types_by_name.get(name).map(|x| { *x })
    }
    pub fn dep_prop(&self, dep_type: DepType<I>, name: &str) -> Option<DepProp<I>> {
        self.dep_types[dep_type.0.index].props_by_name.get(name).map(|x| { *x })
    }
    pub fn reg_val_type(&mut self, desc: Box<ValTypeDesc<I>>) -> ValType<I> {
        self.val_types.push(desc);
        let val_type = ValType(ValTypeI { index: self.val_types.len() - 1 }, PhantomData);
        let name = self.val_types[val_type.0.index].name();
        match self.val_types_by_name.entry(String::from(name)) {
            Occupied(_) => { panic!("The '{}' value type is already registered.", name); }
            Vacant(entry) => entry.insert(val_type)
        };
        val_type
    }
    pub fn reg_dep_type(&mut self, name: String, base: Option<DepType<I>>, ctor: Option<Box<Fn(&Arc<DepObj<I>>, &Fw<I>) + Send>>) -> DepType<I> {
        self.dep_types.push(DepTypeDesc {
            base: base, name: name, props: Vec::new(), props_by_name: HashMap::new(), prop_class: HashMap::new(),
            ctor: ctor
        });
        let dep_type = DepType(DepTypeI { index: self.dep_types.len() - 1 }, PhantomData);
        let name = &self.dep_types[dep_type.0.index].name;
        match self.dep_types_by_name.entry(name.clone()) {
            Occupied(_) => { panic!("The '{}' dependency type is already registered.", name); }
            Vacant(entry) => entry.insert(dep_type)
        };
        dep_type
    }
    fn dep_prop_class(&mut self, target: DepType<I>, prop: DepProp<I>) -> Either<OccupiedDepPropClassRef<I>, &mut DepPropClass<I>> {
        match self.dep_types.get_mut(target.0.index).unwrap().prop_class.entry(prop) {
            Occupied(entry) => Left(OccupiedDepPropClassRef { entry: entry }),
            Vacant(entry) => Right(entry.insert(DepPropClass { def_val: None, set_lock: None, on_changed: Vec::new() }))
        }
    }
    pub fn lock_class_set(&mut self, target: DepType<I>, prop: DepProp<I>) -> ClassSetLock {
        if target.is_locked(prop, self) { panic!("Property setter is class-locked already."); }
        let mut class = self.dep_prop_class(target, prop);
        let lock = ClassSetLock(Unique::new());
        replace(&mut class.set_lock, Some(lock.clone()));
        lock
    }
    pub fn reg_dep_prop(&mut self, owner: DepType<I>, name: String, val_type: Type<I>, def_val: Obj<I>, attached: Option<DepType<I>>) -> DepProp<I> {
        if !def_val.is(&val_type, self) { panic!("Default value type mismatch."); }
        let dep_prop = {
            let owner_desc = &mut self.dep_types[owner.0.index];
            owner_desc.props.push(DepPropDesc { name: name, val_type: val_type, attached: attached });
            let dep_prop = DepProp(DepPropI { owner: owner.0, index: owner_desc.props.len() - 1 }, PhantomData);
            let name = &owner_desc.props[dep_prop.0.index].name;
            match owner_desc.props_by_name.entry(name.clone()) {
                Occupied(_) => { panic!("The '{}' dependency property is already registered for '{}' type.", name, &owner_desc.name); }
                Vacant(entry) => entry.insert(dep_prop)
            };
            dep_prop
        };
        let mut class = self.dep_prop_class(attached.unwrap_or(owner), dep_prop);
        if class.def_val.is_some() { panic!("DEF_VAL_EXISTS"); }
        replace(&mut class.def_val, Some(def_val));
        dep_prop
    }
    pub fn override_def_val(&mut self, dep_type: DepType<I>, dep_prop: DepProp<I>, def_val: Obj<I>) {
        assert_dep_prop_target(dep_prop, dep_type, self);
        assert_dep_prop_val(dep_prop, def_val.type_(), self);
        let mut class = self.dep_prop_class(dep_type, dep_prop);
        if class.def_val.is_some() { panic!("Default value is registered already."); }
        replace(&mut class.def_val, Some(def_val));
    }
    pub fn on_changed(&mut self, dep_type: DepType<I>, dep_prop: DepProp<I>, callback: Box<Fn(&Arc<DepObj<I>>, &Obj<I>, &Obj<I>, &Fw<I>) + Send>) {
        assert_dep_prop_target(dep_prop, dep_type, self);
        let mut class = self.dep_prop_class(dep_type, dep_prop);
        class.on_changed.push(callback);
    }
}

#[macro_export]
macro_rules! fw_instance {
    ($guard_name:ident) => {
        use std;
        use fw;
        pub struct $guard_name(());
        pub type Fw = fw::Fw<$guard_name>;
        lazy_static! {
            pub static ref FW: std::sync::Mutex<Fw> = std::sync::Mutex::new(Fw::new($guard_name(())));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::replace;
    use std::ops::DerefMut;
    use std::sync::{ Arc, Mutex };
    use fw;
    use fw::{ ValTypeDesc, DepObjDataKey };
    pub use fw::Obj::Val as Obj_Val;
    pub use fw::Type::Val as Type_Val;

    struct StrValTypeDesc { }
    impl<I> ValTypeDesc<I> for StrValTypeDesc {
        fn name(&self) -> &str { &"str" }
        fn parse(&self, type_: fw::ValType<I>, s: &str) -> Option<Arc<fw::Val<I>>> {
            Some(type_.box_(String::from(s)))
        }
        fn to_string(&self, val: &fw::Val<I>) -> String { val.unbox::<String>().clone() }
    }

    pub struct TestFw(());
    pub type Fw = fw::Fw<TestFw>;

    lazy_static! {
        static ref FW: Mutex<Fw> = Mutex::new(Fw::new(TestFw(())));
    }

    #[test]
    fn reg_val_type_test() {
        replace(FW.lock().unwrap().deref_mut(), Fw::new(TestFw(())));
        let mut fw = FW.lock().unwrap();
        let str_type = fw.reg_val_type(Box::new(StrValTypeDesc { }));
        assert_eq!("123", str_type.parse(&"123", &fw).unwrap().unbox::<String>());
        assert_eq!("123", str_type.box_(String::from("123")).to_string(&fw));
        assert_eq!("str", str_type.name(&fw));
    }

    #[test]
    fn reg_dep_type_prop_get_set_test() {
        replace(FW.lock().unwrap().deref_mut(), Fw::new(TestFw(())));
        let mut fw = FW.lock().unwrap();
        let str_type = fw.reg_val_type(Box::new(StrValTypeDesc { }));
        let obj_type = fw.reg_dep_type(String::from("obj"), None, None);
        let name_prop = fw.reg_dep_prop(obj_type, String::from("name"), Type_Val(str_type), Obj_Val(str_type.box_(String::from("x"))), None);
        assert_eq!("name", name_prop.name(&fw));
        let obj = obj_type.create(&fw);
        assert_eq!("x", obj.get(name_prop, &fw).unbox::<String>());
        obj.set(name_prop, Obj_Val(str_type.box_(String::from("local value"))), &fw).unwrap();
        assert_eq!("local value", obj.get(name_prop, &fw).unbox::<String>());
        obj.reset(name_prop, &fw).unwrap();
        assert_eq!("x", obj.get(name_prop, &fw).unbox::<String>());
        obj.reset(name_prop, &fw).unwrap();
        assert_eq!("x", obj.get(name_prop, &fw).unbox::<String>());
        let lock = fw.lock_class_set(obj_type, name_prop);
        assert_eq!(Err(()), obj.reset(name_prop, &fw));
        obj.reset_locked(name_prop, &lock, &fw).unwrap();
    }

    #[test]
    fn is_base() {
        replace(FW.lock().unwrap().deref_mut(), Fw::new(TestFw(())));
        let mut fw = FW.lock().unwrap();
        let base_type = fw.reg_dep_type(String::from("base"), None, None);
        let obj_type = fw.reg_dep_type(String::from("obj"), Some(base_type), None);
        assert!(obj_type.is(base_type, &fw));
        assert!(obj_type.is(obj_type, &fw));
        assert!(base_type.is(base_type, &fw));
        assert!(!base_type.is(obj_type, &fw));
    }

    #[test]
    fn lock_set() {
        replace(FW.lock().unwrap().deref_mut(), Fw::new(TestFw(())));
        let mut fw = FW.lock().unwrap();
        let str_type = fw.reg_val_type(Box::new(StrValTypeDesc { }));
        let base_type = fw.reg_dep_type(String::from("base"), None, None);
        let obj_type = fw.reg_dep_type(String::from("obj"), Some(base_type), None);
        let prop = fw.reg_dep_prop(base_type, String::from("Prop"), Type_Val(str_type), Obj_Val(str_type.box_(String::from(""))), None);
        let obj = obj_type.create(&fw);
        assert_eq!("", obj.get(prop, &fw).unbox::<String>());
        obj.set(prop, Obj_Val(str_type.box_(String::from("123"))), &fw).unwrap();
        assert_eq!("123", obj.get(prop, &fw).unbox::<String>());
        let lock = fw.lock_class_set(base_type, prop);
        assert_eq!(Err(()), obj.set(prop, Obj_Val(str_type.box_(String::from("123"))), &fw));
        assert_eq!(Err(()), obj.set(prop, Obj_Val(str_type.box_(String::from("234"))), &fw));
        assert_eq!("123", obj.get(prop, &fw).unbox::<String>());
        obj.set_locked(prop, Obj_Val(str_type.box_(String::from("234"))), &lock, &fw);
        assert_eq!("234", obj.get(prop, &fw).unbox::<String>());
    }

    #[test]
    fn depobj_data() {
        replace(FW.lock().unwrap().deref_mut(), Fw::new(TestFw(())));
        let mut fw = FW.lock().unwrap();
        let obj_type = fw.reg_dep_type(String::from("obj"), None, None);
        let obj = obj_type.create(&fw);
        let key = DepObjDataKey::new();
        assert!(obj.get_data(&key).borrow().is_none());
        obj.set_data(key.clone(), Box::new(13 as i32));
        assert_eq!(13 as i32, *obj.get_data(&key).borrow().unwrap().downcast_ref::<i32>().unwrap());
    }

    #[test]
    fn ctor_test() {
        replace(FW.lock().unwrap().deref_mut(), Fw::new(TestFw(())));
        let mut fw = FW.lock().unwrap();
        let base_value = DepObjDataKey::new();
        let base_type = {
            let base_value = base_value.clone();
            fw.reg_dep_type(String::from("base"), None, Some(Box::new(move |obj, _fw| {
                obj.set_data(base_value.clone(), Box::new(18 as i32));
            })))
        };
        let obj_type = {
            let base_value = base_value.clone();
            fw.reg_dep_type(String::from("obj"), Some(base_type), Some(Box::new(move |obj, _fw| {
                let base = *obj.get_data(&base_value).borrow().unwrap().downcast_ref::<i32>().unwrap();
                obj.set_data(base_value.clone(), Box::new(base + 1));
            })))
        };
        let obj = obj_type.create(&fw);
        let value = *obj.get_data(&base_value).borrow().unwrap().downcast_ref::<i32>().unwrap();
        assert_eq!(19 as i32, value);
    }
}
