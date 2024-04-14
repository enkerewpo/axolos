#include <stdio.h>
#include <stdlib.h>

int a[10];

int main()
{
    printf("Welcome to AXOLOS rootfs init!\n");
    for (int i = 0; i < 10; i++)
    {
        a[i] = i;
    }
    for (int i = 0; i < 10; i++)
    {
        printf("a[%d] = %d\n", i, a[i]);
    }
    while (1)
        ;
}