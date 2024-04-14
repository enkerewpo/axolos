# a build script for axolotl os
# will generate a live cd iso for amd64

ARCH=amd64
HOST_ARCH=$(uname -m)
OUTPUT_DIR=output
ISO_NAME=axolotl-os-$ARCH.iso

LINUX_KERNEL_TARBALL_DL_URL=https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.8.6.tar.xz
LINUX_KERNEL_VERSION="6.8.6"
LINUX_KERNEL_SRC_DIR=$OUTPUT_DIR"/kernel/linux-$LINUX_KERNEL_VERSION"

# check whether the script is running on a amd64 machine, otherwise exit
if [ $HOST_ARCH != "x86_64" ]; then
    echo "This script is only supported on amd64 machines"
    exit 1
fi

build_rootfs() {
    echo "Building rootfs for $ARCH"
    # just add a txt file for now
    mkdir -p $OUTPUT_DIR"/rootfs"
    echo "Hello, this is axolotl os" >$OUTPUT_DIR"/rootfs/README.txt"
    # compile init.c and add it to rootfs
    gcc -static -o $OUTPUT_DIR"/rootfs/init" init.c
    # now let create the dev structure
    mkdir -p $OUTPUT_DIR"/rootfs/dev"
    sudo mknod -m 666 $OUTPUT_DIR"/rootfs/dev/null" c 1 3
    sudo mknod -m 666 $OUTPUT_DIR"/rootfs/dev/tty c" 5 0
    sudo mknod -m 666 $OUTPUT_DIR"/rootfs/dev/zero" c 1 5
    sudo mknod -m 666 $OUTPUT_DIR"/rootfs/dev/random" c 1 8
    sudo mknod -m 666 $OUTPUT_DIR"/rootfs/dev/urandom" c 1 9
    # console
    sudo mknod -m 600 $OUTPUT_DIR"/rootfs/dev/console" c 5 1
    cd $OUTPUT_DIR"/rootfs"
    find . | cpio -H newc -o >../rootfs.cpio
    cd ..
    gzip -c rootfs.cpio >rootfs.cpio.gz
    cd ..
    echo "Finished building rootfs for $ARCH"
}

build_kernel() {
    echo "Building kernel for $ARCH, src at $LINUX_KERNEL_SRC_DIR"
    echo "Finished building kernel for $ARCH"
    # set gcc and other stuff according to the arch
    if [ $ARCH == "amd64" ]; then
        echo "Building kernel for amd64"
        make -C $LINUX_KERNEL_SRC_DIR defconfig
        make -C $LINUX_KERNEL_SRC_DIR -j$(nproc)
        # create an bootable iso
        cp $LINUX_KERNEL_SRC_DIR/arch/x86/boot/bzImage $OUTPUT_DIR"/kernel/bzImage"
        cp $LINUX_KERNEL_SRC_DIR/System.map $OUTPUT_DIR"/kernel/System.map"
        cp $LINUX_KERNEL_SRC_DIR/.config $OUTPUT_DIR"/kernel/.config"
        # create the iso with grub
        mkdir -p $OUTPUT_DIR"/iso"
        cp $OUTPUT_DIR"/kernel/bzImage" $OUTPUT_DIR"/iso"
    else
        echo "Unsupported architecture for now: $ARCH"
    fi
}

build_world() {
    echo "Building world for $ARCH, target is $OUTPUT_DIR"/"$ISO_NAME"
    mkdir -p $OUTPUT_DIR
    mkdir -p $OUTPUT_DIR"/kernel"
    mkdir -p $OUTPUT_DIR"/rootfs"
    # if linux.tar.xz exists, skip downloading
    if [ -f $OUTPUT_DIR"/linux.tar.xz" ]; then
        echo "linux.tar.xz exists, skipping download and extracting as it is already done"
    else
        echo "Downloading linux kernel tarball"
        # download linux kernel and unzip it into kernel directory
        wget $LINUX_KERNEL_TARBALL_DL_URL -O $OUTPUT_DIR"/linux.tar.xz"
        tar -xf $OUTPUT_DIR"/linux.tar.xz" -C $OUTPUT_DIR"/kernel"
    fi
    # the folder name is linux-$LINUX_KERNEL_VERSION under kernel directory
    build_rootfs
    build_kernel
    echo "Finished building world for $ARCH"
}

run_qemu() {
    echo "Running qemu for $ARCH"
    # run qemu with the iso
    qemu-system-x86_64 -kernel $OUTPUT_DIR"/kernel/bzImage" -append "console=ttyS0" -nographic -serial mon:stdio -initrd $OUTPUT_DIR"/rootfs.cpio.gz"
}

help() {
    echo "Usage: $0 [rootfs|kernel|qemu]"
    echo "rootfs: build the rootfs"
    echo "kernel: build the kernel"
    echo "qemu: run qemu with the kernel and rootfs"
}

# check args
if [ $# -eq 0 ]; then
    help
else
    if [ $1 == "rootfs" ]; then
        build_rootfs
    elif [ $1 == "kernel" ]; then
        build_kernel
    elif [ $1 == "qemu" ]; then
        run_qemu
    elif [ $1 == "world" ]; then
        build_world
    else
        help
    fi
fi
