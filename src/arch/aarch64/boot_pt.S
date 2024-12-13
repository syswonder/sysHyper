	PTE_TABLE_FLAGS =0x3 //11
	// PAGE_DEFAULT_FLAG=0x741 //11101000001
    // PAGE_DEVICE_FLAG=0x745 //11101000101

    //PAGE_DEFAULT_FLAG=0x701 //111 0000 0001
    //PAGE_DEVICE_FLAG=0x705 //
                            
    PAGE_DEVICE_FLAG=0x601  //0110 0000 0001
    PAGE_DEFAULT_FLAG=0x705 //0111 0000 0101
    // MAIR_FLAG=0x004404ff //0100 0100 0000 0100 1111 1111
    MAIR_FLAG=0x0044ff00    //0100 0100 1111 1111 0000 0000
    SCTLR_FLAG=0x30c51835 //0111 0000 1100 0101 0001 1000 0011 0101
                          //
    TCR_FLAG=0x80853510 //1000 0000 1000 0101 0011 0101 0001 0000
                        //0000 0000 0000 0101 0011 0101 0001 1001
    .extern uart_puts_c

    .section .bss
    .align 12 // 对齐到页边界
boot_pt_l0:
    .zero 0x1000

    .align 12
boot_pt_l1:
    .zero 0x1000

    .section .text
    .global set_boot_pt

boot_pt_init:
    adrp x1, boot_pt_l0
    adrp x2, boot_pt_l1
    add x2, x2, PTE_TABLE_FLAGS
    str x2, [x1]                      // set boot_pt_l0, index=0 -> boot_pt_l1

    adrp x1, boot_pt_l1
    ldr x2, =(0x0 | PAGE_DEVICE_FLAG) // device memory at 0x0 ~ 0x40000000
    str x2, [x1]
    ldr x2, =(0x40000000 | PAGE_DEFAULT_FLAG) // RAM at 0x40000000 ~ 0x80000000
    str x2, [x1, #0x8]
    ldr x2, =(0x80000000 | PAGE_DEFAULT_FLAG) // RAM at 0x80000000 ~ 0xC0000000
    str x2, [x1, #0x10]
    ldr x2, =(0xC0000000 | PAGE_DEVICE_FLAG) // RAM at 0xC0000000 ~ 0x100000000
    str x2, [x1, #0x18]

    ret

enable_boot_pt:
    stp	x29, x30, [sp, #-0x10]!
    mov	x29, sp

    adrp x1, boot_pt_l0
    msr ttbr0_el2, x1

    isb
	tlbi alle2
    dsb	nsh

    // enable mmu...
    ldr	x1, =MAIR_FLAG     
    msr	mair_el2, x1       // memory attributes for pagetable
    ldr	x1, =TCR_FLAG
	msr	tcr_el2, x1        // translate control, virt range = [0, 2^48)

	/* Enable MMU, allow cacheability for instructions and data */
	ldr	x1, =SCTLR_FLAG
    // bl uart_puts_c
	msr	sctlr_el2, x1      // system control register
    // bl uart_puts_c

	isb
	tlbi alle2
	dsb	nsh

    ldp	x29, x30, [sp], #0x10

    ret


test_ldaxr:
    MOV X0, #0              // 初始化地址为 0
    LDR X1, =shared_var     // 加载全局变量的地址到 X1
retry:
    LDAXR X2, [X1]          // 从地址 X1 读取排他值到 X2
    ADD X2, X2, #1          // 修改值
    STLXR W3, X2, [X1]      // 尝试存储值，W3 返回状态

    CBNZ W3, retry              // 检查存储是否成功
    ret

.section .data
shared_var:
    .word 0                 // 初始化全局变量为 0