/* Linker script for the STM32L053C8 */
MEMORY
{
	/* NOTE 1 K = 1 KiBi = 1024 bytes */
	FLASH : ORIGIN = 0x08000000, LENGTH = 64K
	RAM : ORIGIN = 0x20000000, LENGTH = 8K
}
