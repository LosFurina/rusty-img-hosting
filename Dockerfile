FROM nginx
COPY nginx.conf /etc/nginx/nginx.conf
COPY src/public /usr/share/nginx/html


SHELL ["/bin/bash", "-c"]

LABEL maintainer="liweijun0302@gmail.com"
# Set the working directory
WORKDIR /app
COPY target/release /app

# Expose the port the app runs on
EXPOSE 8080
# Run the application
CMD ["/app/rusty_img_hosting", ">> /tmp/logs/rusty_img_hosting.log 2>&1 &"]
CMD ["nginx"]