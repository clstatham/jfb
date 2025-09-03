#include <stdio.h>
#include "foo.h"

int main() {
    foo();
    printf("Hello, World!\n");
    return 0;
}

void foo() {
    printf("foo() called\n");
}