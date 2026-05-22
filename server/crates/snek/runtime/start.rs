use std::convert::TryInto;
use std::env;


#[repr(align(16))]
struct AlignedHeap([u64; 100000]);
static mut HEAP: AlignedHeap = AlignedHeap([0; 100000]);

#[link(name = "our_code")]
extern "C" {
    // The \x01 here is an undocumented feature of LLVM that ensures
    // it does not add an underscore in front of the name.
    // Courtesy of Max New (https://maxsnew.com/teaching/eecs-483-fa22/hw_adder_assignment.html)
    #[link_name = "\x01our_code_starts_here"]
    fn our_code_starts_here(input: i64, heap: *mut u64) -> i64;
}

                      // 0x0000 -> num
const TRUE: i64 = 7;  // 0x0111
const FALSE: i64 = 3; // 0x0011
const NIL: i64 = 9;   // 0x1001
                      // 0x0001 -> vec 

fn main() {
    let args: Vec<String> = env::args().collect();
    let input = parse_input(&args);
    let i: i64 = unsafe { our_code_starts_here(input, HEAP.0.as_mut_ptr()) };
    snek_print(i);
}

fn parse_input(v: &Vec<String>) -> i64 {
    if v.len() <= 1 {
        return 0;
    }
    let s = &v[1];
    if s == "true" {
        TRUE
    } else if s == "false" {
        FALSE
    } else {
        s.parse::<i64>().unwrap() << 1
    }
}

#[export_name = "\x01snek_error"]
pub extern "C" fn snek_error(errcode: i64) {
    let msg = match errcode {
        1 => format!("overflow"),
        2 => format!("invalid argument: wrong type"),
        3 => format!("invalid argument: conditional must be type `bool`"),
        4 => format!("invalid argument: type mismatch; must be same type"),
        5 => format!("index out of bounds"),
        6 => format!("out of heap space"),
        _ => format!("Undefined error code")
    };

    eprintln!("Error: code {errcode} with msg -- {msg}");
    std::process::exit(1);
}

fn snek_print_val(val: i64) {
    if val == FALSE {
        print!("false");
    } else if val == TRUE {
        print!("true");
    } else if val & 1 == 0 {
        print!("{}", val >> 1);
    } else if val == NIL {
        print!("nil");
    } else {
        // let ptr: *const i64 = unsafe { mem::transmute::<i64, *const i64>(val - 1) };
        let ptr: *const i64 =
            std::ptr::with_exposed_provenance::<i64>((val - 1).try_into().unwrap());
        let len = unsafe { *ptr } as usize;
        print!("(vec");
        for i in 0..len {
            let elem = unsafe { *ptr.add(i + 1) };
            print!(" ");
            snek_print_val(elem);
        }
        print!(")");
    }
}

#[export_name = "\x01snek_print"]
fn snek_print(val: i64) -> i64 {
    snek_print_val(val);
    println!();
    val
}