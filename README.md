# capcap

https://github.com/drbh/capcap/assets/9896130/28947bec-efe7-49d9-99d2-9a294f0de5b2

## Description

Ever wished for your computer to describe a picture's caption? AI makes it possible now!

## Stack

- [blip](https://blog.salesforceairesearch.com/blip-bootstrapping-language-image-pretraining/) vision model
- [candle](https://github.com/huggingface/candle) to run the model
- [poem](https://github.com/poem-web/poem) as http server
- vanilla js for frontend
- deployed without GPU (single shared-1x-cpu@1024MB)

Try it out at [capcap.drbh.xyz](https://capcap.drbh.xyz)
