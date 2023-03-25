: is-mul-of-3       3 %; 0 =; ,,
: is-mul-of-5       5 %; 0 =; ,,
: is-mul-of-3-and-5 dup; is-mul-of-3; swap; is-mul-of-5; and; ,,
: fizzbuzz
    100 range; [1 +] map;
    [      dup; is-mul-of-3-and-5; if; drop; "FizzBuzz"
     else; dup; is-mul-of-3;       if; drop; "Fizz"
     else; dup; is-mul-of-5;       if; drop; "Buzz"
     then;
     then;
     then;] map; ,,
fizzbuzz;
