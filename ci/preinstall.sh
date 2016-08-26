  if [[ "$TRAVIS_OS_NAME" == "osx" ]]; then
    brew update
    brew install python
    pip install virtualenv
    virtualenv venv -p python
    source venv/bin/activate
    pip install pip --upgrade
    pip install travis-cargo
fi