# Neoplayer Ultimate

Rewrite of the original [Neoplayer](https://github.com/arrudagates/neoplayer), this time focusing on spotify instead of youtube (with youtube support coming too) and in Rust instead of JavaScript + Node

## Usage

When running for the first time, follow the instructions printed in the terminal to authenticate with spotify

Press 'e' to exit
Press 'q' to toggle the queue list
Press 'h' to enter input mode
Commands currently available are:
```text
search <query> // Searches spotify for the query and returns a list of results
play <query> // Searches spotify for the query and plays the first result without displaying them
library // Fetches the user's saved songs
pause // Toggle between paused and unpaused states
```
When not in input mode, use arrows up and down to select tracks in the results list and press enter to play them

## Contributing
Feel free to open issues and make pull requests, I'll do my best to work on them.

## License
[GNU General Public License v2.0](LICENSE)
