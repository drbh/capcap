echo "Creating swap file..."
fallocate -l 2048M /swapfile

echo "Setting up swap file..."
chmod 0600 /swapfile

echo "Setting up swap space..."
mkswap /swapfile

echo "Activating swap file..."
sysctl -w vm.swappiness=10

echo "Enabling swap file..."
swapon /swapfile