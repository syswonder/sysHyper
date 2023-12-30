#ifndef __HVISOR_H
#define __HVISOR_H
#include <linux/ioctl.h>
#include <linux/types.h>
#define MMAP_SIZE 1024
#define MAX_REQ 4

#define SIGHVI 10
#define HVISOR_INIT_VIRTIO  _IO(1, 0) // virtio device init
#define HVISOR_GET_TASK _IO(1, 1)	
#define HVISOR_FINISH _IO(1, 2)		  // finish one virtio req	

// receive request from el2
struct device_req {
	__u64 src_cpu;
	__u64 address; // cell's ipa
	__u64 size;
	__u64 value;
	__u32 src_cell;
	__u8 is_write;
	__u8 is_cfg;
};

// return request result to el2
struct device_result {
	__u64 src_cpu;
	__u64 value;
	__u8 is_cfg;
};

struct hvisor_device_region {
	__u32 nreq;
	struct device_req req_list[MAX_REQ];
	struct device_result res;
};



// hypercall
#define HVISOR_CALL_NUM_RESULT "x0"
#define HVISOR_CALL_ARG1       "x1"
#define HVISOR_CALL_INS        "hvc #0x4a48"

#define HVISOR_HC_INIT_VIRTIO 9
#define HVISOR_HC_FINISH_REQ 10

static inline __u64 hvisor_call(__u64 num)
{
	register __u64 num_result asm(HVISOR_CALL_NUM_RESULT) = num;

	asm volatile(
		HVISOR_CALL_INS
		: "=r" (num_result)
		: "r" (num_result)
		: "memory");
	return num_result;
}

static inline __u64 hvisor_call_arg1(__u64 num, __u64 arg1)
{
	register __u64 num_result asm(HVISOR_CALL_NUM_RESULT) = num;
	register __u64 __arg1 asm(HVISOR_CALL_ARG1) = arg1;

	asm volatile(
		HVISOR_CALL_INS
		: "=r" (num_result)
		: "r" (num_result), "r" (__arg1)
		: "memory");
	return num_result;
}

#endif /* __HVISOR_H */