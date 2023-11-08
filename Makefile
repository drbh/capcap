PROJECT_ID := capcap
TAG := latest

fadd-app:
	flyctl apps create $(PROJECT_ID)

fadd-ip:
	flyctl ips allocate-v4 -a $(PROJECT_ID)

fbuild:
	docker buildx build \
	--push -t registry.fly.io/$(PROJECT_ID):$(TAG) .

ftag:
	docker tag drbh/$(PROJECT_ID) registry.fly.io/$(PROJECT_ID):$(TAG)

fpush:
	docker push registry.fly.io/$(PROJECT_ID):$(TAG)

# $5.70/mo
fdeploy-min:
	flyctl m run -a $(PROJECT_ID) \
	--memory 1024 \
	--cpus 1 \
	-p 443:3000/tcp:http:tls \
	registry.fly.io/$(PROJECT_ID):$(TAG)

# $10.70/mo
fdeploy:
	flyctl m run -a $(PROJECT_ID) \
	--memory 2048 \
	--cpus 1 \
	-p 443:3000/tcp:http:tls \
	registry.fly.io/$(PROJECT_ID):$(TAG)

fadd-volume:
	flyctl volumes create data --size 4 -a $(PROJECT_ID)

fip:
	flyctl ips allocate-v4 -a $(PROJECT_ID)

fdestroy:
	flyctl m destroy --select --force -a $(PROJECT_ID)

fpop:
	fly ssh console -s -a $(PROJECT_ID)%    
