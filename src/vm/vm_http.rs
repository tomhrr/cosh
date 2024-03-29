use std::boxed::Box;
use std::cell::RefCell;
use std::rc::Rc;

use futures::executor::block_on;
use http::Method;
use mime::Mime;
use reqwest::blocking::{Client, Response};
use reqwest::header::CONTENT_TYPE;
use url::Url;

use crate::chunk::{Value, new_string_value};
use crate::vm::*;

impl VM {
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
                        _ => {}
                    }
                }
            }
        }

        let text_res = response.text();
        match text_res {
            Ok(text) => {
                self.stack.push(new_string_value(text));
            }
            Err(e) => {
                let err_str = format!("unable to convert response to text: {}", e);
                self.print_error(&err_str);
                return 0;
            }
        }

        return 1;
    }

    pub fn send_request(&mut self, url: &str) -> i32 {
        let client = Client::new();
        let response_res = client.get(url).send();
        match response_res {
            Ok(response) => {
                self.process_response(response)
            }
            Err(e) => {
                let err_str = format!("unable to send request: {}", e);
                self.print_error(&err_str);
                0
            }
        }
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
                    self.send_request(&s2)
                } else {
                    self.send_request(s)
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
                                let url_obj_opt = Url::parse(url_str);
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

                let client = Client::new();
                let mut rb = client.request(method, url);
                let mut is_json = false;

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
                                            if k == "Content-Type" {
                                                let ct_str_opt = Some(v_str);
                                                if let Some(ct_str) = ct_str_opt {
                                                    if let Ok(mct) = Mime::from_str(ct_str) {
                                                        match (mct.subtype(), mct.suffix()) {
                                                            (mime::JSON, _) | (_, Some(mime::JSON)) => {
                                                                is_json = true;
                                                            }
                                                            _ => {}
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            self.print_error("HTTP header value must be string");
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

                let response_res = rb.send();
                match response_res {
                    Ok(response) => {
                        self.process_response(response)
                    }
                    Err(e) => {
                        let err_str = format!("unable to send request: {}", e);
                        self.print_error(&err_str);
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
