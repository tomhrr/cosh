use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::io::BufRead;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use chunk::{print_error, Chunk, Value};
use vm::VM;

impl VM {
    /// Takes a list and an index as its arguments.  Gets the element
    /// at the given index from the list and places it onto the stack.
    pub fn core_nth(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "nth requires two arguments");
            return 0;
        }

        let index_rr = self.stack.pop().unwrap();
        let index_rrb = index_rr.borrow();
        let index_int_opt = index_rrb.to_int();

        let lst_rr = self.stack.pop().unwrap();
        let lst_rrb = lst_rr.borrow();

        match (index_int_opt, &*lst_rrb) {
            (Some(index), Value::List(lst)) => {
                let element = lst[index as usize].clone();
                self.stack.push(element);
            }
            (Some(_), _) => {
                print_error(chunk, i, "first nth argument must be list");
                return 0;
            }
            (_, _) => {
                print_error(chunk, i, "second nth argument must be integer");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a list, an index, and a value as its arguments.  Places
    /// the value at the given index in the list.
    pub fn core_nth_em(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 3 {
            print_error(chunk, i, "nth! requires three arguments");
            return 0;
        }

        let val_rr = self.stack.pop().unwrap();

        let index_rr = self.stack.pop().unwrap();
        let index_rrb = index_rr.borrow();
        let index_int_opt = index_rrb.to_int();

        let lst_rr = self.stack.pop().unwrap();

        {
            let mut lst_rrb = lst_rr.borrow_mut();
            match (index_int_opt, &mut *lst_rrb) {
                (Some(index), Value::List(lst)) => {
                    lst[index as usize] = val_rr;
                }
                (Some(_), _) => {
                    print_error(chunk, i, "first nth! argument must be list");
                    return 0;
                }
                (_, _) => {
                    print_error(chunk, i, "second nth! argument must be integer");
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
        scopes: &mut Vec<RefCell<HashMap<String, Rc<RefCell<Value>>>>>,
        global_functions: &mut RefCell<HashMap<String, Chunk>>,
        prev_local_vars_stacks: &mut Vec<Rc<RefCell<Vec<Rc<RefCell<Value>>>>>>,
        chunk: &Chunk, i: usize, line_col: (u32, u32),
        running: Arc<AtomicBool>,
    ) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "gnth requires two arguments");
            return 0;
        }

        let index_rr = self.stack.pop().unwrap();
        let index_rrb = index_rr.borrow();
        let index_int_opt = index_rrb.to_int();

        match index_int_opt {
            Some(mut index) => {
                while index >= 0 {
                    let dup_res = self.opcode_dup(chunk, i);
                    if dup_res == 0 {
                        return 0;
                    }
                    let shift_res = self.opcode_shift(
                        scopes,
                        global_functions,
                        prev_local_vars_stacks,
                        chunk,
                        i,
                        line_col,
                        running.clone(),
                    );
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
                print_error(chunk, i, "second gnth argument must be integer");
                return 0;
            }
        }
        return 1;
    }

    /// Takes a list and a value as its arguments.  Pushes the value
    /// onto the list and places the updated list onto the stack.
    pub fn core_push(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "push requires two arguments");
            return 0;
        }

        let element_rr = self.stack.pop().unwrap();
        let lst_rr = self.stack.pop().unwrap();

        {
            let mut lst_rrb = lst_rr.borrow_mut();
            match *lst_rrb {
                Value::List(ref mut lst) => {
                    lst.push_back(element_rr);
                }
                _ => {
                    print_error(chunk, i, "first push argument must be list");
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
    pub fn core_unshift(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 2 {
            print_error(chunk, i, "unshift requires two arguments");
            return 0;
        }

        let element_rr = self.stack.pop().unwrap();
        let lst_rr = self.stack.pop().unwrap();

        {
            let mut lst_rrb = lst_rr.borrow_mut();
            match *lst_rrb {
                Value::List(ref mut lst) => {
                    lst.push_front(element_rr);
                }
                _ => {
                    print_error(chunk, i, "first unshift argument must be list");
                    return 0;
                }
            }
        }

        self.stack.push(lst_rr);
        return 1;
    }

    /// Takes a list as its single argument.  Pops a value from the
    /// end of the list and places that value onto the stack.
    pub fn core_pop(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "pop requires one argument");
            return 0;
        }

        let lst_rr = self.stack.pop().unwrap();
        let element_rr = match *(lst_rr.borrow_mut()) {
            Value::List(ref mut lst) => {
                let element_rr_opt = lst.pop_back();
                match element_rr_opt {
                    Some(element_rr) => element_rr,
                    None => Rc::new(RefCell::new(Value::Null)),
                }
            }
            _ => {
                print_error(chunk, i, "pop argument must be list");
                return 0;
            }
        };

        self.stack.push(element_rr);
        return 1;
    }

    /// Takes a shiftable object as its single argument.  Shifts an
    /// element from that object and puts it onto the stack.
    pub fn opcode_shift<'a>(
        &mut self,
        scopes: &mut Vec<RefCell<HashMap<String, Rc<RefCell<Value>>>>>,
        global_functions: &mut RefCell<HashMap<String, Chunk>>,
        prev_local_vars_stacks: &mut Vec<Rc<RefCell<Vec<Rc<RefCell<Value>>>>>>,
        chunk: &Chunk, i: usize, line_col: (u32, u32),
        running: Arc<AtomicBool>,
    ) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "shift requires one argument");
            return 0;
        }

        let mut repush = false;
        let mut stack_len = 0;
        let mut new_stack_len = 0;

        let shiftable_rr = self.stack.pop().unwrap();
        {
            let mut shiftable_rrb = shiftable_rr.borrow_mut();
            match *shiftable_rrb {
                Value::Generator(
                    ref mut global_vars,
                    ref mut local_vars_stack,
                    ref mut index,
                    ref chunk,
                    ref call_stack_chunks,
                    ref mut gen_args,
                    ref mut chunk_values,
                ) => {
                    stack_len = self.stack.len();
                    let mut is_empty = false;
                    if gen_args.len() == 1 {
                        let gen_arg_rr = &gen_args[0];
                        let gen_arg_rrb = gen_arg_rr.borrow();
                        match &*gen_arg_rrb {
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
                        self.stack.push(Rc::new(RefCell::new(Value::Int(0))));
                    } else {
                        let gen_args_len = gen_args.len();
                        if gen_args_len > 0 {
                            while gen_args.len() > 0 {
                                self.stack.push(gen_args.pop().unwrap());
                            }
                            self.stack.push(Rc::new(RefCell::new(Value::Int(
                                gen_args_len as i32,
                            ))));
                        }
                    }

                    let mut call_stack_chunks_sub = Vec::new();
                    for i in call_stack_chunks.iter() {
                        call_stack_chunks_sub.push(i);
                    }
                    let current_index = *index;
                    if current_index == chunk.data.borrow().len() {
                        /* At end of function: push null. */
                        self.stack.push(Rc::new(RefCell::new(Value::Null)));
                    } else {
                        let res = self.run(
                            scopes,
                            global_functions,
                            &call_stack_chunks_sub,
                            chunk,
                            chunk_values,
                            *index,
                            Some(global_vars),
                            Some(local_vars_stack),
                            prev_local_vars_stacks,
                            line_col,
                            running,
                        );
                        match res {
                            0 => {
                                return 0;
                            }
                            i => {
                                *index = i;
                                new_stack_len = self.stack.len();
                                repush = true;
                            }
                        }
                    }
                }
                Value::List(ref mut lst) => {
                    if lst.len() > 0 {
                        let value_rr = lst.pop_front().unwrap();
                        self.stack.push(value_rr);
                    } else {
                        self.stack.push(Rc::new(RefCell::new(Value::Null)));
                    }
                }
                Value::CommandGenerator(ref mut bufread) => {
                    let mut contents = String::new();
                    let res = BufRead::read_line(bufread, &mut contents);
                    match res {
                        Ok(bytes) => {
                            if bytes != 0 {
                                self.stack.push(Rc::new(RefCell::new(
                                    Value::String(contents, None),
                                )));
                            } else {
                                self.stack
                                    .push(Rc::new(RefCell::new(Value::Null)));
                            }
                        }
                        Err(_) => {
                            print_error(
                                chunk,
                                i,
                                "unable to read next line from command output",
                            );
                        }
                    }
                }
                Value::KeysGenerator(ref mut index, ref mut hash_rr) => {
                    let hash_rrb = hash_rr.borrow();
                    match &*hash_rrb {
                        Value::Hash(map) => {
                            let kv = map.get_index(*index);
                            match kv {
                                Some((k, _)) => {
                                    self.stack.push(Rc::new(RefCell::new(
                                        Value::String(k.to_string(), None),
                                    )));
                                }
                                None => {
                                    self.stack.push(Rc::new(RefCell::new(
                                        Value::Null,
                                    )));
                                }
                            }
                            *index = *index + 1;
                        }
                        _ => {
                            eprintln!(
                                "keys generator does not contain a hash!"
                            );
                            std::process::abort();
                        }
                    }
                }
                Value::ValuesGenerator(ref mut index, ref mut hash_rr) => {
                    let hash_rrb = hash_rr.borrow();
                    match &*hash_rrb {
                        Value::Hash(map) => {
                            let kv = map.get_index(*index);
                            match kv {
                                Some((_, v)) => {
                                    self.stack.push(v.clone());
                                }
                                None => {
                                    self.stack.push(Rc::new(RefCell::new(
                                        Value::Null,
                                    )));
                                }
                            }
                            *index = *index + 1;
                        }
                        _ => {
                            eprintln!(
                                "values generator does not contain a hash!"
                            );
                            std::process::abort();
                        }
                    }
                }
                Value::EachGenerator(ref mut index, ref mut hash_rr) => {
                    let hash_rrb = hash_rr.borrow();
                    match &*hash_rrb {
                        Value::Hash(map) => {
                            let kv = map.get_index(*index);
                            match kv {
                                Some((k, v)) => {
                                    let mut lst = VecDeque::new();
                                    lst.push_back(Rc::new(RefCell::new(
                                        Value::String(k.to_string(), None),
                                    )));
                                    lst.push_back(v.clone());
                                    self.stack.push(Rc::new(RefCell::new(
                                        Value::List(lst),
                                    )));
                                }
                                None => {
                                    self.stack.push(Rc::new(RefCell::new(
                                        Value::Null,
                                    )));
                                }
                            }
                            *index = *index + 1;
                        }
                        _ => {
                            eprintln!(
                                "each generator does not contain a hash!"
                            );
                            std::process::abort();
                        }
                    }
                }
                _ => {
                    print_error(chunk, i, "argument cannot be shifted");
                    return 0;
                }
            }
        }
        if repush {
            if new_stack_len == stack_len {
                self.stack.push(Rc::new(RefCell::new(Value::Null)));
            }
        }

        return 1;
    }

    /// Takes a shiftable object as its single argument, and places
    /// copies of all the elements from the list onto the stack, in
    /// the order that they are shifted.
    pub fn core_shift_all(
        &mut self,
        scopes: &mut Vec<RefCell<HashMap<String, Rc<RefCell<Value>>>>>,
        global_functions: &mut RefCell<HashMap<String, Chunk>>,
        prev_local_vars_stacks: &mut Vec<Rc<RefCell<Vec<Rc<RefCell<Value>>>>>>,
        chunk: &Chunk, i: usize, line_col: (u32, u32),
        running: Arc<AtomicBool>
    ) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "shift-all requires one argument");
            return 0;
        }

        loop {
            let dup_res = self.opcode_dup(chunk, i);
            if dup_res == 0 {
                return 0;
            }
            let shift_res = self.opcode_shift(
                scopes,
                global_functions,
                prev_local_vars_stacks,
                chunk,
                i,
                line_col,
                running.clone(),
            );
            if shift_res == 0 {
                self.stack.pop();
                return 0;
            }
            let is_null;
            {
                let shifted_rr = &self.stack[self.stack.len() - 1];
                let shifted_rrb = shifted_rr.borrow();
                match &*shifted_rrb {
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

            let swap_res = self.opcode_swap(chunk, i);
            if swap_res == 0 {
                return 0;
            }
        }
        return 1;
    }

    /// Takes an arbitrary value as its single argument.  Places a
    /// boolean onto the stack indicating whether the argument can be
    /// shifted.
    pub fn opcode_isshiftable(&mut self, chunk: &Chunk, i: usize) -> i32 {
        if self.stack.len() < 1 {
            print_error(chunk, i, "is-shiftable requires one argument");
            return 0;
        }

        let value_rr = self.stack.pop().unwrap();
        let value_rrb = value_rr.borrow();
        let res = match *value_rrb {
            Value::List(_) => 1,
            Value::Generator(_, _, _, _, _, _, _) => 1,
            Value::CommandGenerator(_) => 1,
            Value::KeysGenerator(_, _) => 1,
            Value::ValuesGenerator(_, _) => 1,
            Value::EachGenerator(_, _) => 1,
            _ => 0,
        };
        self.stack.push(Rc::new(RefCell::new(Value::Int(res))));
        return 1;
    }
}
