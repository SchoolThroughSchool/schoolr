#!/bin/bash

sudo truncate -s 0 /swapfile
sudo chattr +C /swapfile

sudo fallocate -l 10G /swapfile
sudo chmod 600 /swapfile

# Make it to swap format and activate on your system
sudo mkswap /swapfile
sudo swapon /swapfile

sudo apt-get update
sudo apt-get install -y libssl-dev pkg-config
