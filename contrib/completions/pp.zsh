#compdef pp

_pp_services() {
	local -a services
	services=("${(@f)$(pp __complete services 2>/dev/null)}")
	_describe 'service' services
}

_pp() {
	local context state line
	typeset -A opt_args

	_arguments -C \
		'1:command:(list add run r info i set unset remove rename doctor edit path completions open stop logs status)' \
		'2:service:->service' \
		'*::arg:->args'

	case "$state" in
		service)
			case "$words[2]" in
				run|r|info|i|set|unset|remove|doctor|open|stop|logs|status)
					_pp_services
					;;
			esac
			;;
	esac
}

_pp "$@"
