(function() {
	if (typeof(Oculus) === 'undefined') {
		window.Oculus = {};
	}

	var AI = Oculus.AI = function () {

	};

	AI.prototype.selectBase = function (bases) {
		var largestBase = {value: 0};
		for (var i = 0; i < bases.length; i++) {
			if (bases[i].value > largestBase.value && bases[i].team === 'cpu') {
				largestBase = bases[i];
			}
		}
		return largestBase;
	};

	AI.prototype.attackBase = function (bases, selectedBase) {
		var shuffledBases = this.FYShuffle(bases);
		if (selectedBase.value < 10) return undefined;
		for (var i = 0; i < bases.length; i++) {
			if (shuffledBases[i].team !== 'cpu'
				&& (selectedBase.value * (Math.random() + 0.33 )) > shuffledBases[i].value) {
				return shuffledBases[i];
			}
		}
		return undefined;
	};

	AI.prototype.FYShuffle = function (array) {
	  var currentIndex = array.length, temporaryValue, randomIndex;

	  // While there remain elements to shuffle...
	  while (0 !== currentIndex) {

	    // Pick a remaining element...
	    randomIndex = Math.floor(Math.random() * currentIndex);
	    currentIndex -= 1;

	    // And swap it with the current element.
	    temporaryValue = array[currentIndex];
	    array[currentIndex] = array[randomIndex];
	    array[randomIndex] = temporaryValue;
	  }

	  return array;
	};

})();
