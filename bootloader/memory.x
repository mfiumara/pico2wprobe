MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  /* Bootloader region */
  FLASH                             : ORIGIN = 0x10000000, LENGTH = 64K
  /* Boot state partition - stores swap metadata */
  BOOTLOADER_STATE                  : ORIGIN = 0x10010000, LENGTH = 4K
  /* Active firmware partition - currently running app */
  ACTIVE                            : ORIGIN = 0x10011000, LENGTH = 1M
  /* DFU partition - staged firmware updates (must be >= ACTIVE + 1 page) */
  DFU                               : ORIGIN = 0x10111000, LENGTH = 1M + 4K

  /* RP2350 has 520KB SRAM: 512K striped + 8K direct mapped */
  RAM   : ORIGIN = 0x20000000, LENGTH = 512K
  SRAM4 : ORIGIN = 0x20080000, LENGTH = 4K
  SRAM5 : ORIGIN = 0x20081000, LENGTH = 4K
}

/* Bootloader partition offsets (relative to flash base 0x10000000) */
__bootloader_state_start = ORIGIN(BOOTLOADER_STATE) - ORIGIN(FLASH);
__bootloader_state_end = ORIGIN(BOOTLOADER_STATE) + LENGTH(BOOTLOADER_STATE) - ORIGIN(FLASH);

__bootloader_active_start = ORIGIN(ACTIVE) - ORIGIN(FLASH);
__bootloader_active_end = ORIGIN(ACTIVE) + LENGTH(ACTIVE) - ORIGIN(FLASH);

__bootloader_dfu_start = ORIGIN(DFU) - ORIGIN(FLASH);
__bootloader_dfu_end = ORIGIN(DFU) + LENGTH(DFU) - ORIGIN(FLASH);

/* RP2350 Boot ROM sections */
SECTIONS {
    /* ### Boot ROM info
     *
     * Goes after .vector_table, to keep it in the first 4K of flash
     * where the Boot ROM (and picotool) can find it
     */
    .start_block : ALIGN(4)
    {
        __start_block_addr = .;
        KEEP(*(.start_block));
        KEEP(*(.boot_info));
    } > FLASH

} INSERT AFTER .vector_table;

/* move .text to start /after/ the boot info */
_stext = ADDR(.start_block) + SIZEOF(.start_block);

SECTIONS {
    /* ### Picotool 'Binary Info' Entries
     *
     * Picotool looks through this block (as we have pointers to it in our
     * header) to find interesting information.
     */
    .bi_entries : ALIGN(4)
    {
        /* We put this in the header */
        __bi_entries_start = .;
        /* Here are the entries */
        KEEP(*(.bi_entries));
        /* Keep this block a nice round size */
        . = ALIGN(4);
        /* We put this in the header */
        __bi_entries_end = .;
    } > FLASH
} INSERT AFTER .uninit;

SECTIONS {
    /* ### Boot ROM extra info
     *
     * Goes after everything in our program, so it can contain a signature.
     */
    .end_block : ALIGN(4)
    {
        __end_block_addr = .;
        KEEP(*(.end_block));
    } > FLASH

} INSERT AFTER .bi_entries;

PROVIDE(start_to_end = __end_block_addr - __start_block_addr);
PROVIDE(end_to_start = __start_block_addr - __end_block_addr);
