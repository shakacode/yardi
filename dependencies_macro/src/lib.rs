#![recursion_limit="128"]

extern crate syn;
extern crate quote;
extern crate proc_macro;
extern crate proc_macro2;

use self::proc_macro::{TokenStream};
use proc_macro2::Span;
use syn::{Type, TypePath, Ident, Expr, LitBool, ExprClosure};
use quote::*;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{braced, Token, bracketed};

struct Import {

}

struct Alias {

}

impl Parse for Alias {
    fn parse(_input: ParseStream) -> Result<Self> {
        Ok(Alias { })
    }
}

struct Constant {
    class: Type,
    name: Ident,
    value: Expr
}

impl Parse for Constant {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;

        let class = input.parse()?;
        input.parse::<Token![=]>()?;
        
        let value = input.parse()?;
        Ok(Constant { class, name, value })
    }
}

struct Dependency {
    name: Ident,
    constructor: ConstructorKind,
    class: TypePath,
    singleton: bool,
    args: Vec<Ident>
}

enum ConstructorKind {
    Default,
    Ident(Ident),
    Closure(ExprClosure),
}

enum DependencyItem {
    Struct(TypePath),
    Args(Vec<Ident>),
    Singleton(bool),
    Constructor(ConstructorKind)
}

impl Parse for DependencyItem {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(if input.peek(Token![struct]) {
            input.parse::<Token![struct]>()?;
            input.parse::<Token![=]>()?;

            DependencyItem::Struct(input.parse()?)
        } else {
            {   // error checking
                let fork = input.fork();
                let ident = fork.parse::<Ident>()?;

                let ident_str = ident.to_string();
                if  ident_str.as_str() != "args" &&
                    ident_str.as_str() != "ctor" && 
                    ident_str.as_str() != "singleton" {
                    
                    return Err(input.error("expected `singleton`, `args` or `ctor` keywords"));
                }
            }

            let ident = input.parse::<Ident>()?;
            let ident_str = ident.to_string();

            input.parse::<Token![=]>()?;

            match ident_str.as_str() {
                "singleton" => {
                    let val: LitBool = input.parse()?;
                    DependencyItem::Singleton(val.value)
                }

                "args" => {
                    let mut args = Vec::new();
                    let mut args_stream;

                    bracketed!(args_stream in input);

                    let x: Punctuated<Ident, Token![,]> = Punctuated::parse_terminated(&mut args_stream)?;

                    for pair in x.into_pairs() {
                        args.push(pair.into_value());
                    }

                    DependencyItem::Args(args)
                }
                
                "ctor" => {
                    let ctor = if input.peek(Token![|]) || input.peek(Token![move]) {
                        let x = input.parse::<Expr>()?;

                        ConstructorKind::Closure(match x {
                            Expr::Closure(c) => c,
                            _ => unreachable!()
                        })
                    } else if input.peek(Ident) {
                        let ident: Ident = input.parse()?;
                        if ident.to_string().as_str() == "default" {
                            ConstructorKind::Default
                        } else {
                            ConstructorKind::Ident(ident)
                        }
                    } else {
                        return Err(input.error("Expected Closure or Ident"));
                    };

                    DependencyItem::Constructor(ctor)
                }
                _ => unreachable!()
            }
        })
    }
}

impl Parse for Dependency {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = Vec::new();
        let name: Ident = input.parse()?;
        let mut constructor = ConstructorKind::Ident(Ident::new("new", Span::call_site()));
        let mut singleton = true;

        let class = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            input.parse()?
        } else {
            let mut inner;
            braced!(inner in input);
            let mut class = None;

            let items: Punctuated<DependencyItem, Token![,]> = Punctuated::parse_terminated(&mut inner)?;

            for pair in items.into_pairs() {
                match pair.into_value() {
                    DependencyItem::Struct(path) => class = Some(path),
                    DependencyItem::Constructor(name) => constructor = name,
                    DependencyItem::Args(a) => args.extend(a.into_iter()),
                    DependencyItem::Singleton(val) => singleton = val,
                };
            }

            match class {
                Some(c) => c,
                None => return Err(input.error("struct is required field"))
            }
        };

        Ok(Dependency { name, class, args, constructor, singleton })
    }
}

#[derive(Default)]
struct Dependencies {
    services: Vec<Dependency>,
    aliases: Vec<Alias>,
    consts: Vec<Constant>,
}

enum DependenciesEntry {
    Services(Vec<Dependency>),
    Aliases(Vec<Alias>),
    Consts(Vec<Constant>),
}

impl Parse for DependenciesEntry {
    fn parse(input: ParseStream) -> Result<Self> {
        let fork = input.fork();
        let ident: Ident = fork.parse()?;
        let ident_str = ident.to_string();

        match ident_str.as_str() {
            "services" => {
                let _ = input.parse::<Ident>()?;
                let mut content;
                braced!(content in input);

                let mut services = Vec::new();
                let x: Punctuated<Dependency, Token![,]> = Punctuated::parse_terminated(&mut content)?;
                for p in x.into_pairs() {
                    services.push(p.into_value())
                }

                Ok(DependenciesEntry::Services(services))
            }

            "consts" => {
                let _ = input.parse::<Ident>()?;
                
                let mut content;
                braced!(content in input);

                let mut consts = Vec::new();
                let x: Punctuated<Constant, Token![,]> = Punctuated::parse_terminated(&mut content)?;
                for p in x.into_pairs() {
                    consts.push(p.into_value())
                }

                Ok(DependenciesEntry::Consts(consts))
            }

            "aliases" => return Err(input.error("unimplemented!")),
            _ => return Err(input.error("Expected `services`, `consts` or `aliases` keyword!"))
        }
    }
}


impl Parse for Dependencies {
    fn parse(mut input: ParseStream) -> Result<Self> {
        let x: Punctuated<DependenciesEntry, Token![,]> = Punctuated::parse_terminated(&mut input)?;

        let mut deps = Dependencies::default();

        for p in x.into_pairs() {
            match p.into_value() {
                DependenciesEntry::Consts(c) => deps.consts = c,
                DependenciesEntry::Services(s) => deps.services = s,
                _ => return Err(input.error("unimplemented"))
            }
        }

        Ok(deps)
    }
}

#[proc_macro]
pub fn dependencies(input: TokenStream) -> TokenStream {
    let toks = match syn::parse::<Dependencies>(input) {
        Ok(deps) => {
            let mut inject_fields = proc_macro2::TokenStream::new();
            let mut inject_impls = proc_macro2::TokenStream::new();
            let mut deps_decls = proc_macro2::TokenStream::new();

            for cnst in deps.consts.iter() {
                let class = &cnst.class;
                let name = &cnst.name;
                let value = &cnst.value;

                let field = quote! {
                    #[allow(non_snake_case)]
                    #name: std::sync::Mutex<InjectorCell<<deps::#name as Dep>::DependecyType>>,
                };

                let dep_decl = quote! {
                    #[derive(Debug)]
                    #[allow(non_camel_case_types)]
                    pub struct #name;
                    impl Dep for #name {
                        type DependecyType = #class;
                    }
                };

                let dep_impl = quote! {
                    impl Inject<deps::#name> for Injector {
                        fn inject(&self) -> <deps::#name as Dep>::DependecyType {
                            match &mut *self.#name.lock().unwrap() {
                                InjectorCell::Some(val) => val.clone(),
                                InjectorCell::Pending => unreachable!(),
                                this @ InjectorCell::None => {
                                    let valx: <deps::#name as Dep>::DependecyType = #value;
                                    let clone = valx.clone();

                                    *this = InjectorCell::Some(valx);

                                    clone
                                }
                            }
                        }
                    }
                };

                field.to_tokens(&mut inject_fields);
                dep_impl.to_tokens(&mut inject_impls);
                dep_decl.to_tokens(&mut deps_decls);
            }

            for dep in deps.services.iter() {
                let name = &dep.name;
                let class = &dep.class;
                let singleton = dep.singleton;

                let field = quote! {
                    #[allow(non_snake_case)]
                    #name: std::sync::Mutex<InjectorCell<<deps::#name as Dep>::DependecyType>>,
                };

                field.to_tokens(&mut inject_fields);

                let dep_decl = quote! {
                    #[derive(Debug, Copy, Clone)]
                    #[allow(non_camel_case_types)]
                    pub struct #name;
                    impl Dep for #name {
                        type DependecyType = #class;
                    }
                };

                let mut dep_args = proc_macro2::TokenStream::new();
                let mut arg_types = proc_macro2::TokenStream::new();
                for arg in dep.args.iter() {
                    let arg_decl = quote! { 
                        <Injector as Inject<deps::#arg>>::inject(self),
                    };

                    let arg_type = quote! {
                        <deps::#arg as Dep>::DependecyType,
                    };

                    arg_decl.to_tokens(&mut dep_args);
                    arg_type.to_tokens(&mut arg_types);
                }

                let ctor = match &dep.constructor {
                    ConstructorKind::Default => quote! {
                        Default::default()
                    },

                    ConstructorKind::Ident(ident) => quote! {
                        <deps::#name as Dep>::DependecyType::#ident(#dep_args)
                    },

                    ConstructorKind::Closure(closure) => quote! {{
                        fn check<R, F: Fn(#arg_types) -> R>(f: F) -> F { f }

                        check({#closure})(#dep_args)
                    }},
                };

                let dep_impl = if singleton {
                    quote! {
                        impl Inject<deps::#name> for Injector {
                            fn inject(&self) -> <deps::#name as Dep>::DependecyType {
                                match &mut *self.#name.lock().unwrap() {
                                    InjectorCell::Pending => panic!("Curcular dependecy referencing!"),
                                    this @ InjectorCell::None => {
                                        *this = InjectorCell::Pending;

                                        let valx: <deps::#name as Dep>::DependecyType = #ctor;

                                        *this = InjectorCell::None;

                                        valx
                                    },
                                    _ => unreachable!()
                                }
                            }
                        }
                    }
                } else {
                    quote! {
                        impl Inject<deps::#name> for Injector {
                            fn inject(&self) -> <deps::#name as Dep>::DependecyType {
                                match &mut *self.#name.lock().unwrap() {
                                    InjectorCell::Some(val) => val.clone(),
                                    InjectorCell::Pending => panic!("Curcular dependecy referencing!"),
                                    this @ InjectorCell::None => {
                                        *this = InjectorCell::Pending;

                                        let valx: <deps::#name as Dep>::DependecyType = #ctor;
                                        let clone = valx.clone();

                                        *this = InjectorCell::Some(valx);

                                        clone
                                    }
                                }
                            }
                        }
                    }
                };

                
                dep_impl.to_tokens(&mut inject_impls);
                dep_decl.to_tokens(&mut deps_decls);
            }

            quote! {
                pub use crate::injector::utils::{Dep, Inject, InjectorError};

                pub mod deps {
                    use super::*;

                    #deps_decls
                }

                enum InjectorCell<T> {
                    Some(T),
                    Pending,
                    None
                }

                impl<T> Default for InjectorCell<T> {
                    fn default() -> Self {
                        InjectorCell::None
                    }
                }

                #[derive(Default)]
                pub struct Injector {
                    #inject_fields
                }

                impl Injector {
                    pub fn new() -> Self {
                        Default::default()
                    }
                }

                #inject_impls
            }
        }

        Err(err) => err.to_compile_error()
    };

    toks.into()
}

