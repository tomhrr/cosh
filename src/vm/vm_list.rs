use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::BufRead;
use std::mem;
use std::rc::Rc;

use chunk::{StringPair, Value};
use vm::VM;

impl VM {
    /// Takes a list and an index as its arguments.  Gets the element
    /// at the given index from the list and places it onto the stack.
    pub fn core_nth(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("nth requires two arguments");
            return 0;
        }

        let index_rr = self.stack.pop().unwrap();
        let index_int_opt = index_rr.to_int();

        let lst_rr = self.stack.pop().unwrap();

        match (index_int_opt, lst_rr) {
            (Some(index), Value::List(lst)) => {
                if lst.borrow().len() <= (index as usize) {
                    self.print_error("nth index is out of bounds");
                    return 0;
                }
                let element = lst.borrow()[index as usize].clone();
                self.stack.push(element);
            }
            (Some(_), _) => {
                self.print_error("first nth argument must be list");
                return 0;
            }
            (_, _) => {
                self.print_error("second nth argument must be integer");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a list, an index, and a value as its arguments.  Places
    /// the value at the given index in the list.
    pub fn core_nth_em(&mut self) -> i32 {
        if self.stack.len() < 3 {
            self.print_error("nth! requires three arguments");
            return 0;
        }

        let val_rr = self.stack.pop().unwrap();

        let index_rr = self.stack.pop().unwrap();
        let index_int_opt = index_rr.to_int();

        let mut lst_rr = self.stack.pop().unwrap();

        {
            match (index_int_opt, &mut lst_rr) {
                (Some(index), Value::List(lst)) => {
                    if lst.borrow().len() <= (index as usize) {
                        self.print_error("nth! index is out of bounds");
                        return 0;
                    }
                    lst.borrow_mut()[index as usize] = val_rr;
                }
                (Some(_), _) => {
                    self.print_error("first nth! argument must be list");
                    return 0;
                }
                (_, _) => {
                    self.print_error("second nth! argument must be integer");
                    return 0;
                }
            }
        }
        self.stack.push(lst_rr);
        return 1;
    }

    /// Takes a generator and an index as its arguments.  Gets the
    /// element at the given index from the generator and places it
    /// onto the stack.
    pub fn core_gnth(
        &mut self,
    ) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("gnth requires two arguments");
            return 0;
        }

        let index_rr = self.stack.pop().unwrap();
        let index_int_opt = index_rr.to_int();

        match index_int_opt {
            Some(mut index) => {
                while index >= 0 {
                    let dup_res = self.opcode_dup();
                    if dup_res == 0 {
                        return 0;
                    }
                    let shift_res = self.opcode_shift();
                    if shift_res == 0 {
                        return 0;
                    }
                    if index == 0 {
                        self.stack.remove(self.stack.len() - 2);
                        break;
                    } else {
                        self.stack.pop();
                        index = index - 1;
                    }
                }
            }
            _ => {
                self.print_error("second gnth argument must be integer");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a list and a value as its arguments.  Pushes the value
    /// onto the list and places the updated list onto the stack.
    pub fn opcode_push(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("push requires two arguments");
            return 0;
        }

        let element_rr = self.stack.pop().unwrap();
        let mut lst_rr = self.stack.pop().unwrap();

        {
            match lst_rr {
                Value::List(ref mut lst) => {
                    lst.borrow_mut().push_back(element_rr);
                }
                _ => {
                    self.print_error("first push argument must be list");
                    return 0;
                }
            }
        }

        self.stack.push(lst_rr);
        return 1;
    }

    /// Takes a list and a value as its arguments.  Pushes the value
    /// onto the start of the list and places the updated list onto
    /// the stack.
    pub fn core_unshift(&mut self) -> i32 {
        if self.stack.len() < 2 {
            self.print_error("unshift requires two arguments");
            return 0;
        }

        let element_rr = self.stack.pop().unwrap();
        let mut lst_rr = self.stack.pop().unwrap();

        {
            match lst_rr {
                Value::List(ref mut lst) => {
                    lst.borrow_mut().push_front(element_rr);
                }
                _ => {
                    self.print_error("first unshift argument must be list");
                    return 0;
                }
            }
        }

        self.stack.push(lst_rr);
        return 1;
    }

    /// Takes a list as its single argument.  Pops a value from the
    /// end of the list and places that value onto the stack.
    pub fn opcode_pop(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("pop requires one argument");
            return 0;
        }

        let mut lst_rr = self.stack.pop().unwrap();
        let element_rr = match lst_rr {
            Value::List(ref mut lst) => {
                let element_rr_opt = lst.borrow_mut().pop_back();
                match element_rr_opt {
                    Some(element_rr) => element_rr,
                    None => Value::Null,
                }
            }
            _ => {
                self.print_error("pop argument must be list");
                return 0;
            }
        };

        self.stack.push(element_rr);
        return 1;
    }

    pub fn opcode_shift_inner<'a>(
        &mut self,
        shiftable_rr: &mut Value
    ) -> i32 {
        let mut repush = false;
        let mut stack_len = 0;
        let mut new_stack_len = 0;

        match *shiftable_rr {
            Value::Generator(ref mut generator_object_) => {
                // todo: set to none, error later if still none.
                let mut new_i = 0;
                {
                    let mut generator_object = generator_object_.borrow_mut();
                    let index = generator_object.index;

                    {
                        let gen_args = &mut generator_object.gen_args;
                        stack_len = self.stack.len();
                        let mut is_empty = false;
                        if gen_args.len() == 1 {
                            let gen_arg_rr = &gen_args[0];
                            match gen_arg_rr {
                                Value::Null => {
                                    is_empty = true;
                                }
                                _ => {
                                    is_empty = false;
                                }
                            }
                        }
                        if is_empty {
                            gen_args.pop();
                            self.stack.push(Value::Int(0));
                        } else {
                            let gen_args_len = gen_args.len();
                            if gen_args_len > 0 {
                                while gen_args.len() > 0 {
                                    self.stack.push(gen_args.pop().unwrap());
                                }
                                self.stack.push(Value::Int(gen_args_len as i32));
                            }
                        }
                    }

                    /* todo: need a generator-specific run function,
                     * to avoid the stuffing around here. */
                    let global_vars = generator_object.global_vars.clone();
                    let local_vars_stack = generator_object.local_vars_stack.clone();
                    let chunk = generator_object.chunk.clone();
                    let call_stack_chunks = &mut generator_object.call_stack_chunks;
                    mem::swap(call_stack_chunks, &mut self.call_stack_chunks);

                    let current_index = index;
                    if current_index == chunk.borrow().data.len() {
                        /* At end of function: push null. */
                        self.stack.push(Value::Null);
                    } else {
                        self.scopes.push(global_vars);
                        let plvs_stack = self.local_var_stack.clone();
                        self.local_var_stack = local_vars_stack;
                        let backup_chunk = self.chunk.clone();
                        self.chunk = chunk.clone();
                        let i = self.i;
                        self.i = index;
                        let res = self.run_inner();
                        self.i = i;
                        self.chunk = backup_chunk;
                        self.scopes.pop();
                        self.local_var_stack = plvs_stack;
                        mem::swap(call_stack_chunks, &mut self.call_stack_chunks);
                        match res {
                            0 => {
                                return 0;
                            }
                            i => {
                                new_i = i;
                                new_stack_len = self.stack.len();
                                repush = true;
                            }
                        }
                    }
                }
                generator_object_.borrow_mut().index = new_i;
            }
            Value::List(ref mut lst) => {
                if lst.borrow().len() > 0 {
                    let value_rr = lst.borrow_mut().pop_front().unwrap();
                    self.stack.push(value_rr);
                } else {
                    self.stack.push(Value::Null);
                }
            }
            Value::CommandGenerator(ref mut bufread) => {
                let mut contents = String::new();
                let res = bufread.borrow_mut().read_line(&mut contents);
                match res {
                    Ok(bytes) => {
                        if bytes != 0 {
                            self.stack.push(Value::String(Rc::new(RefCell::new(
                                StringPair::new(contents, None),
                            ))));
                        } else {
                            self.stack.push(Value::Null);
                        }
                    }
                    Err(_) => {
                        self.print_error("unable to read next line from command output");
                    }
                }
            }
            Value::KeysGenerator(ref mut hwi) => {
                {
                    let hash_rr = &hwi.borrow().h;
                    match hash_rr {
                        Value::Hash(map) => {
                            let mapb = map.borrow();
                            let kv = mapb.get_index(hwi.borrow().i);
                            match kv {
                                Some((k, _)) => {
                                    self.stack.push(Value::String(Rc::new(RefCell::new(
                                        StringPair::new(k.to_string(), None),
                                    ))));
                                }
                                None => {
                                    self.stack.push(Value::Null);
                                }
                            }
                        }
                        _ => {
                            eprintln!("keys generator does not contain a hash!");
                            std::process::abort();
                        }
                    }
                }
                let el = hwi.borrow().i + 1;
                hwi.borrow_mut().i = el;
            }
            Value::ValuesGenerator(ref mut hwi) => {
                {
                    let hash_rr = &hwi.borrow().h;
                    match hash_rr {
                        Value::Hash(map) => {
                            let mapb = map.borrow();
                            let kv = mapb.get_index(hwi.borrow().i);
                            match kv {
                                Some((_, v)) => {
                                    self.stack.push(v.clone());
                                }
                                None => {
                                    self.stack.push(Value::Null);
                                }
                            }
                        }
                        _ => {
                            eprintln!("values generator does not contain a hash!");
                            std::process::abort();
                        }
                    }
                }
                let el = hwi.borrow().i + 1;
                hwi.borrow_mut().i = el;
            }
            Value::EachGenerator(ref mut hwi) => {
                {
                    let hash_rr = &hwi.borrow().h;
                    match hash_rr {
                        Value::Hash(map) => {
                            let mapb = map.borrow();
                            let kv = mapb.get_index(hwi.borrow().i);
                            match kv {
                                Some((k, v)) => {
                                    let mut lst = VecDeque::new();
                                    lst.push_back(Value::String(Rc::new(RefCell::new(
                                        StringPair::new(k.to_string(), None),
                                    ))));
                                    lst.push_back(v.clone());
                                    self.stack.push(Value::List(Rc::new(RefCell::new(lst))));
                                }
                                None => {
                                    self.stack.push(Value::Null);
                                }
                            }
                        }
                        _ => {
                            eprintln!("each generator does not contain a hash!");
                            std::process::abort();
                        }
                    }
                }
                let el = hwi.borrow().i + 1;
                hwi.borrow_mut().i = el;
            }
            _ => {
                self.print_error("argument cannot be shifted");
                return 0;
            }
        }
        if repush {
            if new_stack_len == stack_len {
                self.stack.push(Value::Null);
            }
        }

        return 1;
    }


    /// Takes a shiftable object as its single argument.  Shifts an
    /// element from that object and puts it onto the stack.
    pub fn opcode_shift<'a>(
        &mut self,
    ) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("shift requires one argument");
            return 0;
        }

        let mut shiftable_rr = self.stack.pop().unwrap();
        return self.opcode_shift_inner(
            &mut shiftable_rr
        );
    }

    /// Takes a shiftable object as its single argument, and places
    /// copies of all the elements from the list onto the stack, in
    /// the order that they are shifted.
    pub fn core_shift_all(
        &mut self,
    ) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("shift-all requires one argument");
            return 0;
        }

        loop {
            let dup_res = self.opcode_dup();
            if dup_res == 0 {
                return 0;
            }
            let shift_res = self.opcode_shift();
            if shift_res == 0 {
                self.stack.pop();
                return 0;
            }
            let is_null;
            {
                let shifted_rr = &self.stack[self.stack.len() - 1];
                match shifted_rr {
                    Value::Null => {
                        is_null = true;
                    }
                    _ => {
                        is_null = false;
                    }
                }
            }
            if is_null {
                self.stack.pop();
                self.stack.pop();
                break;
            }

            let swap_res = self.opcode_swap();
            if swap_res == 0 {
                return 0;
            }
        }
        return 1;
    }

    /// Takes an arbitrary value as its single argument.  Places a
    /// boolean onto the stack indicating whether the argument can be
    /// shifted.
    pub fn opcode_isshiftable(&mut self) -> i32 {
        if self.stack.len() < 1 {
            self.print_error("is-shiftable requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let res = match value_rr {
            Value::List(_) => 1,
            Value::Generator(_) => 1,
            Value::CommandGenerator(_) => 1,
            Value::KeysGenerator(_) => 1,
            Value::ValuesGenerator(_) => 1,
            Value::EachGenerator(_) => 1,
            _ => 0,
        };
        self.stack.push(Value::Int(res));
        return 1;
    }
}
