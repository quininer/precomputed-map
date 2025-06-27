.section .text.<precomputed_map::MediumMap<precomputed_map::store::ConstSlice<21821942, 333337, [u8; 22155279]>, precomputed_map::aligned::AlignedArray<40408, u32, precomputed_map::store::ConstSlice<8000000, 40408, [u8; 8040408]>>, (precomputed_map::seq::CompactSeq<0, 21821942, precomputed_map::aligned::AlignedArray<4000000, u32, precomputed_map::store::ConstSlice<0, 4000000, [u8; 8040408]>>, [u8; 22155279]>, precomputed_map::aligned::AlignedArray<4000000, u32, precomputed_map::store::ConstSlice<4000000, 4000000, [u8; 8040408]>>), str2id::Fx>>::get::<[u8]>,"ax",@progbits
	.p2align	4
.type	<precomputed_map::MediumMap<precomputed_map::store::ConstSlice<21821942, 333337, [u8; 22155279]>, precomputed_map::aligned::AlignedArray<40408, u32, precomputed_map::store::ConstSlice<8000000, 40408, [u8; 8040408]>>, (precomputed_map::seq::CompactSeq<0, 21821942, precomputed_map::aligned::AlignedArray<4000000, u32, precomputed_map::store::ConstSlice<0, 4000000, [u8; 8040408]>>, [u8; 22155279]>, precomputed_map::aligned::AlignedArray<4000000, u32, precomputed_map::store::ConstSlice<4000000, 4000000, [u8; 8040408]>>), str2id::Fx>>::get::<[u8]>,@function
<precomputed_map::MediumMap<precomputed_map::store::ConstSlice<21821942, 333337, [u8; 22155279]>, precomputed_map::aligned::AlignedArray<40408, u32, precomputed_map::store::ConstSlice<8000000, 40408, [u8; 8040408]>>, (precomputed_map::seq::CompactSeq<0, 21821942, precomputed_map::aligned::AlignedArray<4000000, u32, precomputed_map::store::ConstSlice<0, 4000000, [u8; 8040408]>>, [u8; 22155279]>, precomputed_map::aligned::AlignedArray<4000000, u32, precomputed_map::store::ConstSlice<4000000, 4000000, [u8; 8040408]>>), str2id::Fx>>::get::<[u8]>:
	.cfi_startproc
	push rbp
	.cfi_def_cfa_offset 16
	push r15
	.cfi_def_cfa_offset 24
	push r14
	.cfi_def_cfa_offset 32
	push r13
	.cfi_def_cfa_offset 40
	push r12
	.cfi_def_cfa_offset 48
	push rbx
	.cfi_def_cfa_offset 56
	push rax
	.cfi_def_cfa_offset 64
	.cfi_offset rbx, -56
	.cfi_offset r12, -48
	.cfi_offset r13, -40
	.cfi_offset r14, -32
	.cfi_offset r15, -24
	.cfi_offset rbp, -16
	mov r14, rdx
	mov r15, rsi
	mov rbx, rdi
	mov r13d, dword ptr [rdi + 48]
	mov r12, qword ptr [rdi + 40]
	mov rdi, r12
	call <str2id::Fx as precomputed_map::phf::HashOne>::hash_one::<&[u8]>
	mov ecx, eax
	imul rcx, rcx, 333337
	shr rcx, 32
	; pilots 间接地址访问，应该调整布局来消除*
	mov rdx, qword ptr [rbx]
	movzx ecx, byte ptr [rdx + rcx + 21821942]
	; hash pilot
	xor rcx, r12
	movabs rdx, 5871781006564002453
	imul rdx, rcx
	; h(hash) ^ h(pilot) & l(pilot)
	; rax = hash
	; rdx = pilot
	xor rax, rdx
	shr rax, 32
	mov ecx, edx
	xor rcx, rax
	; reduct
	; r13 = slots_len
	imul rcx, r13
	mov r12, rcx
	shr r12, 32
	; rcx = r12 >> 4
	shr rcx, 38
	cmp ecx, 15625
	; index <= data_len 应该是 hot path*
	jb .LBB0_3
	lea rax, [4*r12 - 3999996]
	; remap 边界检查，应该约束 slots_len = data_len + remap_len*
	cmp rax, 40409
	jae .LBB0_14
	shl r12, 2
	; remap 的间接地址访问
	mov rax, qword ptr [rbx + 8]*
	mov r12d, dword ptr [rax + r12 + 4000000]
.LBB0_3:
	; keys、value 的间接地址访问*
	mov rax, qword ptr [rbx + 16]
	mov rdi, qword ptr [rbx + 24]
	test r12, r12
	je .LBB0_4
	; 不知道为什么做两次 cmp … 应该消除*
	cmp r12d, 1000001
	jae .LBB0_14
	cmp r12, 1000000
	je .LBB0_14
	; 理想情况下应该用 qword 一个指令取出*
	mov ecx, dword ptr [rax + 4*r12 - 4]
	mov eax, dword ptr [rax + 4*r12]
	cmp eax, ecx
	jb .LBB0_15
	cmp eax, 21821942
	ja .LBB0_16
.LBB0_9:
	sub rax, rcx
	xor ebp, ebp
	cmp rax, r14
	jne .LBB0_10
	add rdi, rcx
	mov rsi, r15
	mov rdx, r14
	call qword ptr [rip + bcmp@GOTPCREL]
	test eax, eax
	jne .LBB0_13
	mov rax, qword ptr [rbx + 32]
	mov edx, dword ptr [rax + 4*r12 + 4000000]
	mov ebp, 1
	jmp .LBB0_13
.LBB0_10:
.LBB0_13:
	mov eax, ebp
	add rsp, 8
	.cfi_def_cfa_offset 56
	pop rbx
	.cfi_def_cfa_offset 48
	pop r12
	.cfi_def_cfa_offset 40
	pop r13
	.cfi_def_cfa_offset 32
	pop r14
	.cfi_def_cfa_offset 24
	pop r15
	.cfi_def_cfa_offset 16
	pop rbp
	.cfi_def_cfa_offset 8
	ret
.LBB0_4:
	.cfi_def_cfa_offset 64
	mov eax, dword ptr [rax]
	xor ecx, ecx
	cmp eax, 21821942
	jbe .LBB0_9
.LBB0_16:
	lea rdx, [rip + .Lanon.753a257926c7149674946eabf91d7097.3]
	mov esi, 21821942
	mov rdi, rax
	call qword ptr [rip + core::slice::index::slice_end_index_len_fail@GOTPCREL]
.LBB0_15:
	lea rdx, [rip + .Lanon.753a257926c7149674946eabf91d7097.3]
	mov rdi, rcx
	mov rsi, rax
	call qword ptr [rip + core::slice::index::slice_index_order_fail@GOTPCREL]
.LBB0_14:
	lea rdi, [rip + .Lanon.753a257926c7149674946eabf91d7097.4]
	call qword ptr [rip + core::option::unwrap_failed@GOTPCREL]
