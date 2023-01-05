PNPM_MINIMUM_VERSION=7.15.0

vercomp(){
   local a b IFS=. -; set -f
   printf -v a %08d $1; printf -v b %08d $3
   test $a "$2" $b
}

if ! command -v pnpm &> /dev/null
then
    printf "pnpm not found !\n please run npm i -g pnpm@7.15.0 to install it\n"
    exit 1
fi

PNPM_VERSION=$(pnpm --version)

if vercomp $PNPM_MINIMUM_VERSION \< $PNPM_VERSION; then
   printf  "current pnpm version is too old\n"
   printf  "please update to ${PNPM_MINIMUM_VERSION} or newer!\n"
   echo older
   exit 1
else
   printf  "current pnpm version : ${PNPM_VERSION} is ok\n"
   echo newer
fi

printf "adding env file in project root directory ...\n"
touch ./.env
echo VITE_NAPI_ID=GDL > ./.env
printf "env file created in script running directory! ...\n"

#install dependencies
pnpm i

#switching throug all possible options
case $1 in
  run) ;&
  r)
    # run GD LAUNCHER
    pnpm dev-app
    ;;
  release);&
  rel)
    # build in release mode GD LAUNCHER
    if [ $# -eq 1 ]
    then
        unameOsOut="$(uname -s)"
        case "${unameOsOut}" in
            Linux*)     os=linux;;
            Darwin*)    os=mac;;
            CYGWIN*);&
            MINGW*)     os=win;;
            *)
              printf "unable to reconize running os ${unameOsOut}!\n"
              printf "run ${0} (rel|release) {mac|win|linux}-{arm64|x64}\n"
              exit 1
              ;;
        esac
        printf "detected os : ${os}"

        unameArchOut="$(uname -m)"
        case "${unameArchOut}" in
                    x86_64*)    arch=x64;;
                    aarch64)    arch=arm64;;
                    *)
                      printf "unable to reconize current architecture os ${unameOsOut}!\n"
                      printf "run ${0} (rel|release) {mac|win|linux}-{arm64|x64}\n"
                      exit 1
                      ;;
        esac
        printf "detected architecture : ${arch}"
        release_args=build-${os}-${arch}
    else
      release_args=${2}
    fi
    echo "2 args : ${2}"
    echo "release args : ${release_args}"
    pnpm ${release_args}
    ;;
  test);&
  t)
    # run all the automatic tests of GD LAUNCHER
    pnpm test
    ;;
  lint);&
  l)
    # run linter on GD LAUNCHER
    pnpm lint
    ;;
  *) printf "wrong option ${1}"
     printf "usage ${0}  [(run|r)|(release|rel) target |test ] \n =  ";;
esac
