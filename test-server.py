import logging
from flask import Flask, request
app = Flask(__name__)

@app.route('/', methods=['POST', 'GET'], defaults={'path': ''})
@app.route('/<path:path>', methods=['POST', 'GET'])
def index(path):
    print("HTTP {} to URL /{} received JSON {}".format(request.method, path, request.get_json()))
    return "True"

if __name__ == '__main__':
    app.run(host='127.0.0.1', port=12345, debug=True)