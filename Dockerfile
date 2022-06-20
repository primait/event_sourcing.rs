FROM public.ecr.aws/prima/rust:1.58.0-1

# Serve per avere l'owner dei file scritti dal container uguale all'utente Linux sull'host
USER app

WORKDIR /code

ENTRYPOINT ["/bin/bash"]
