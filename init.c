#include <stdio.h>
#include <stdlib.h>

int main()
{
    printf("Welcome to AXOLOS rootfs init!\n");
    system("/zsh");
    while (1)
        ;
}