extern crate modular_server;
use neon::prelude::*;

fn hello(mut cx: FunctionContext) -> JsResult<JsString> {
    Ok(cx.string("hello node"))
}

fn spawn(mut cx: FunctionContext) -> JsResult<JsString> {
    let x = modular_server::spawn("127.0.0.1:7813".to_owned(), "7812".to_owned());
    Ok(cx.string(format!("x: {:?}", x)))
}

register_module!(mut cx, {
    cx.export_function("hello", hello)?;
    cx.export_function("spawn", spawn)?;
    Ok(())
});
