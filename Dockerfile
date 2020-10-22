FROM ubuntu:18.04
ENV TINI_URL="https://github.com/krallin/tini/releases/download/v0.18.0/tini-static-amd64"
ENV PACKAGES="libssl1.0 ca-certificates"

RUN apt-get -y update \
 && apt-get -y install ${PACKAGES}

ADD ${TINI_URL} /tini
RUN chmod a+x /tini \
    && mkdir -p /app \
    && groupadd -g 10000 pso \
    && useradd -u 10000 -g 10000 -s /bin/sh pso

ENTRYPOINT ["/tini", "--"]

COPY target/release/pull-secret-operator /app/pull-secret-operator
ADD docker/entrypoint.sh /entrypoint.sh
RUN chown -R pso.pso /app \
 && chmod a+x /entrypoint.sh
USER 10000
CMD ["/entrypoint.sh"]
