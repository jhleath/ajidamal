# As it turns out, there is a lovely TFT frame buffer driver for linux
# already for the display that we are using (the adafruit 1.8" TFT SPI
# display).
#
# fbtft - https://github.com/notro/fbtft
#
# This bash script will register the device with the kernel module
# using the correct gpio pinouts. Once this is run, using standard
# tools to write to the framebuffer (like fbi or even X) should begin
# to work normally.

sudo modprobe fbtft_device name=adafruit18 gpios=reset:22,dc:27,led:0 txbuflen=32768
