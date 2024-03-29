use std::boxed::Box;
use std::cell::RefCell;
use std::rc::Rc;

use futures::executor::block_on;
use mime::Mime;
use reqwest::blocking::{Client, Response};
use reqwest::header::CONTENT_TYPE;

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
}
