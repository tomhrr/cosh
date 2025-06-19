use std::cell::RefCell;
use std::rc::Rc;

use http::Method;
use mime::Mime;
use reqwest::blocking::{Client, Response, RequestBuilder};
use reqwest::header::CONTENT_TYPE;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time;
use url::Url;

use crate::chunk::{Value, new_string_value};
use crate::vm::*;

impl VM {
    pub fn response_body_to_value(&mut self, response: Response) -> Value {
        let bytes_res = response.bytes();
        match bytes_res {
            Ok(bytes) => {
                let s_res = String::from_utf8(bytes.to_vec());
                match s_res {
                    Ok(s) => {
                        return new_string_value(s);
                    }
                    _ => {
			let mut lst = VecDeque::new();
			for i in bytes {
			    lst.push_back(Value::Byte(i));
			}
                        return Value::List(Rc::new(RefCell::new(lst)));
                    }
                }
            }
            Err(e) => {
                let err_str = format!("unable to get response body: {}", e);
                self.print_error(&err_str);
                return Value::Null;
            }
        }
    }

    pub fn process_response(&mut self, response: Response) -> i32 {
        let headers = response.headers();
        if let Some(content_type) = headers.get(CONTENT_TYPE) {
            if let Ok(ct_str) = content_type.to_str() {
                if let Ok(mct) = Mime::from_str(ct_str) {
                    match (mct.subtype(), mct.suffix()) {
                        (mime::JSON, _) | (_, Some(mime::JSON)) => {
                            let text_res = response.text();
                            match text_res {
                                Ok(text) => {
                                    self.stack.push(new_string_value(text));
                                    return self.core_from_json();
                                }
                                Err(e) => {
                                    let err_str = format!("unable to convert response to text: {}", e);
                                    self.print_error(&err_str);
                                    return 0;
                                }
                            }
                        }
                        (mime::XML, _) | (_, Some(mime::XML)) => {
                            let text_res = response.text();
                            match text_res {
                                Ok(text) => {
                                    self.stack.push(new_string_value(text));
                                    return self.core_from_xml();
                                }
                                Err(e) => {
                                    let err_str = format!("unable to convert response to text: {}", e);
                                    self.print_error(&err_str);
                                    return 0;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let value = self.response_body_to_value(response);
        match value {
            Value::Null => {
                return 0;
            }
            _ => {
                self.stack.push(value);
            }
        }

        return 1;
    }

    pub fn send_request_simple(&mut self, url: &str) -> i32 {
        let client = Client::new();
        let rb = client.request(Method::GET, url);
        let res = self.send_request(rb);
        match res {
            Some(response) => {
                return self.process_response(response);
            }
            _ => {
                return 0;
            }
        }
    }

    pub fn send_request(&mut self, mut rb: RequestBuilder) -> Option<Response> {
        rb = rb.header("User-Agent",
                       format!("cosh/{}", env!("CARGO_PKG_VERSION")));
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let response_res = rb.send();
            let _ = tx.send(response_res);
        });
        loop {
            let response_recv_res = rx.try_recv();
            match response_recv_res {
                Ok(Ok(response)) => {
                    return Some(response);
                }
                Ok(Err(e)) => {
                    let err_str = format!("unable to send request: {}", e);
                    self.print_error(&err_str);
                    return None;
                }
                Err(TryRecvError::Disconnected) => {
                    let err_str = format!("unable to send request: disconnected");
                    self.print_error(&err_str);
                    return None;
                }
                Err(TryRecvError::Empty) => {
		    if !self.running.load(Ordering::SeqCst) {
			self.running.store(true, Ordering::SeqCst);
			self.stack.clear();
			return None;
		    }
		    let dur = time::Duration::from_secs_f64(0.05);
		    thread::sleep(dur);
                }
            }
        };
    }

    pub fn core_http_get(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("http.get requires one argument");
            return 0;
        }

        let str_rr = self.stack.pop().unwrap();
        let str_opt: Option<&str>;
        to_str!(str_rr, str_opt);

        match str_opt {
            Some(s) => {
                if !s.starts_with("http") {
                    let s2 = "https://".to_owned() + s;
                    self.send_request_simple(&s2)
                } else {
                    self.send_request_simple(s)
                }
            }
            _ => {
                self.print_error("http.get argument must be a string");
                0
            }
        }
    }

    pub fn core_http(&mut self) -> i32 {
        if self.stack.is_empty() {
            self.print_error("http requires one argument");
            return 0;
        }

        let args_rr = self.stack.pop().unwrap();
        match args_rr {
            Value::Hash(map) => {
                let method;
                let url;

                let mapp = map.borrow();
                let method_val_opt = mapp.get("method");
                match method_val_opt {
                    Some(method_val) => {
                        let method_str_opt: Option<&str>;
                        to_str!(method_val, method_str_opt);
                        match method_str_opt {
                            Some(method_str) => {
                                let method_str_uc =
                                    method_str.to_ascii_uppercase();
                                let method_obj_opt =
                                    Method::from_str(&method_str_uc);
                                match method_obj_opt {
                                    Ok(method_obj) => {
                                        method = method_obj;
                                    }
                                    _ => {
                                        self.print_error("HTTP method is invalid");
                                        return 0;
                                    }
                                }
                            }
                            _ => {
                                self.print_error("HTTP method must be string");
                                return 0;
                            }
                        }
                    }
                    _ => {
                        method = Method::GET;
                    }
                }

                let url_val_opt = mapp.get("url");
                match url_val_opt {
                    Some(url_val) => {
                        let url_str_opt: Option<&str>;
                        to_str!(url_val, url_str_opt);
                        match url_str_opt {
                            Some(url_str) => {
                                let url_obj_opt =
                                    if !url_str.starts_with("http") {
                                        let url_str2 = "https://".to_owned() + url_str;
                                        Url::parse(&url_str2)
                                    } else {
                                        Url::parse(url_str)
                                    };
                                match url_obj_opt {
                                    Ok(url_obj) => {
                                        url = url_obj;
                                    }
                                    _ => {
                                        self.print_error("HTTP URL is invalid");
                                        return 0;
                                    }
                                }
                            }
                            _ => {
                                self.print_error("HTTP URL must be string");
                                return 0;
                            }
                        }
                    }
                    _ => {
                        self.print_error("http hash argument must contain URL");
                        return 0;
                    }
                }

                let raw_val_opt = mapp.get("raw");
                let mut raw = false;
                match raw_val_opt {
                    Some(raw_val) => {
                        raw = raw_val.to_bool();
                    }
                    _ => {}
                }

                let redirect_body_val_opt = mapp.get("redirect-body");
                let mut redirect_body = false;
                match redirect_body_val_opt {
                    Some(redirect_body_val) => {
                        redirect_body = redirect_body_val.to_bool();
                    }
                    _ => {}
                }

                let client = if redirect_body {
                    // Create client that doesn't follow redirects
                    Client::builder()
                        .redirect(reqwest::redirect::Policy::none())
                        .build()
                        .unwrap()
                } else {
                    Client::new()
                };
                let mut rb = client.request(method, url);
                let mut is_json = false;
                let mut is_xml  = false;

                rb = rb.header("User-Agent",
                               format!("cosh/{}", env!("CARGO_PKG_VERSION")));

                let headers_val_opt = mapp.get("headers");
                match headers_val_opt {
                    Some(headers_val) => {
                        match headers_val {
                            Value::Hash(hmap) => {
                                let hmapp = hmap.borrow();
                                for (k, v) in hmapp.iter() {
                                    let v_str_opt: Option<&str>;
                                    to_str!(v, v_str_opt);
                                    match v_str_opt {
                                        Some(v_str) => {
                                            rb = rb.header(k, v_str);
                                            if k.to_ascii_lowercase() == "content-type" {
                                                let ct_str_opt = Some(v_str);
                                                if let Some(ct_str) = ct_str_opt {
                                                    if let Ok(mct) = Mime::from_str(ct_str) {
                                                        match (mct.subtype(), mct.suffix()) {
                                                            (mime::JSON, _) | (_, Some(mime::JSON)) => {
                                                                is_json = true;
                                                            }
                                                            (mime::XML, _) | (_, Some(mime::XML)) => {
                                                                is_xml = true;
                                                            }
                                                            _ => {}
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            self.print_error("HTTP header value must be string");
                                            return 0;
                                        }
                                    }
                                }
                            }
                            _ => {
                                self.print_error("HTTP headers must be hash");
                                return 0;
                            }
                        }
                    }
                    _ => {}
                }

                let body_val_opt = mapp.get("body");
                match body_val_opt {
                    Some(body_val) => {
                        if is_json {
                            self.stack.push(body_val.clone());
                            let res = self.core_to_json();
                            if res == 0 {
                                return res;
                            }
                            let body_val_encoded =
                                self.stack.pop().unwrap();
                            let body_str_opt: Option<&str>;
                            to_str!(body_val_encoded, body_str_opt);
                            match body_str_opt {
                                Some(body_str) => {
                                    rb = rb.body(body_str.to_string());
                                }
                                _ => {
                                    self.print_error("unable to process body");
                                    return 0;
                                }
                            }
                        } else if is_xml {
                            self.stack.push(body_val.clone());
                            let res = self.core_to_xml();
                            if res == 0 {
                                return res;
                            }
                            let body_val_encoded =
                                self.stack.pop().unwrap();
                            let body_str_opt: Option<&str>;
                            to_str!(body_val_encoded, body_str_opt);
                            match body_str_opt {
                                Some(body_str) => {
                                    rb = rb.body(body_str.to_string());
                                }
                                _ => {
                                    self.print_error("unable to process body");
                                    return 0;
                                }
                            }
                        } else {
                            let body_str_opt: Option<&str>;
                            to_str!(body_val, body_str_opt);
                            match body_str_opt {
                                Some(body_str) => {
                                    rb = rb.body(body_str.to_string());
                                }
                                _ => {
                                    self.print_error("unable to process body");
                                    return 0;
                                }
                            }
                        }
                    }
                    _ => {}
                }

                let response_res = self.send_request(rb);
                match response_res {
                    Some(response) => {
                        if raw {
                            let mut headers = IndexMap::new();
                            for (k, v) in response.headers().iter() {
                                headers.insert(k.to_string(),
                                               new_string_value((*v).to_str().unwrap().to_string()));
                            }
                            let mut result = IndexMap::new();
                            result.insert("headers".to_string(),
                                          Value::Hash(Rc::new(RefCell::new(headers))));
                            result.insert("code".to_string(),
                                          Value::Int(response.status().as_u16() as i32));
                            let value = self.response_body_to_value(response);
                            result.insert("body".to_string(), value);
                            let hv =
                                Value::Hash(Rc::new(RefCell::new(result)));
                            self.stack.push(hv);
                        } else {
                            self.process_response(response);
                        }
                        1
                    }
                    _ => {
                        0
                    }
                }
            }
            _ => {
                self.print_error("http argument must be hash");
                0
            }
        }
    }
}
