FROM public.ecr.aws/primaassicurazioni/rust:1.96.1

WORKDIR /code

COPY entrypoint /code/entrypoint

RUN cargo install sqlx-cli --no-default-features --features native-tls,postgres --version 0.7.1

RUN chown -R app:app /code

# Needed to have the same file owner in the container and in Linux host
USER app

ENTRYPOINT ["./entrypoint"]
