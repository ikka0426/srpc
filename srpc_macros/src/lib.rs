
//! # srpc_macros
//! 
//! `srpc_macros` is a crate contains a series of macros to generate code, which reduce duplication.
//! 
//! This crate provides macros that can be used to define remote services and their methods.

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{ parse, ItemImpl, Type, ImplItem, FnArg, Pat };

/// defination of remote service and methods
/// 
/// These macros automatically import the following items, so the example code needs to be used in the parent package environment:
/// 
/// - `srpc::server::remote_call::RemoteCall`: Used for remote method invocation.
/// - `srpc::server::remote_call::Error`: Used for error handling.
/// - `std::any::Any`: Used for handling values of various types.
/// 
/// # Examples
/// ```
/// struct Calc;
/// 
/// #[srpc_macros::remote]
/// impl Calc {
///   fn add(x: i32, y: i32) -> i32 {
///     x + y
///   }
///   fn sub(x: i32, y: i32) -> i32 {
///     x - y
///   }
///   fn div(x: i32, y: i32) -> (i32, i32) {
///     (x / y, x % y)
///   }
/// }
/// 
/// let sub_1_2 = Calc::remote_call("sub", Box::new((1, 2))).unwrap().downcast::<i32>().unwrap();
/// assert_eq!(sub_1_2, -1);
/// ```
/// 
#[proc_macro_attribute]
pub fn remote(_: TokenStream, input: TokenStream) -> TokenStream {
  let ast: ItemImpl = parse(input).unwrap();

  let service = if let Type::Path(x) = &*ast.self_ty {
    if let Some(x) = x.path.segments.first() {
      x
    } else {
      unreachable!()
    }
  } else {
    unreachable!()
  };

  
  let service_name_code = &service.ident;
  
  let match_branch_code = ast.items.iter().map(|x| {
    if let ImplItem::Fn(x) = x {
      let method_name_code = &x.sig.ident;
      let (arg_name_code, arg_type_code): (Vec<_>, Vec<_>) = x.sig.inputs.iter().skip(1).map(|x| {
        if let FnArg::Typed(x) = x {
          
          let arg_name_code = if let Pat::Ident(x) = &*x.pat {
            &x.ident
          } else {
            unreachable!()
          };
          
          let arg_type_code = if let Type::Path(x) = &*x.ty {
            if let Some(x) = x.path.segments.first() {
              &x.ident
            } else {
              unreachable!()
            }
          } else {
            unreachable!()
          };
          
          (arg_name_code, arg_type_code)
        } else {
          unreachable!()
        }
      }).unzip();
      
      let arg_type_code_str = quote!(#(#arg_type_code,)*).to_string();
      let arg_type_code_str = arg_type_code_str[..arg_type_code_str.len() - 1].to_string();
      
      let panic_message = format!("Arguments are not of type ({}) for method '{}'", arg_type_code_str, method_name_code);
      
      quote!(
        stringify!(#method_name_code) => {
          let (#(#arg_name_code,)*) = match serde_json::from_value::<(#(#arg_type_code,)*)>(args) {
            Ok(value) => value,
            Err(_) => return Err(srpc::server::remote_call::Error::ArgumentsNotMatchError(#panic_message.to_string()))
          };

          Ok(serde_json::to_value(self.#method_name_code(#(#arg_name_code,)*)).unwrap())
        }
      )
    } else {
      unreachable!()
    }
  });



  let res = quote!(
    
    #ast
    
    impl srpc::server::remote_call::RemoteCall for #service_name_code {
      fn call(&self, method: &str, args: serde_json::Value) -> Result<serde_json::Value, srpc::server::remote_call::Error> {
        match method {
          #(#match_branch_code)*
          _ => {
            Err(srpc::server::remote_call::Error::NoSuchMethodError)
          }
        }
      }
    }

  ).into();

  println!("{}", res);
  res

}