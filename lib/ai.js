(function() {
	if (typeof(Oculus) === 'undefined') {
		window.Oculus = {};
	}

	var AI = Oculus.AI = function () {
		this.power = 1;
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
		// Difficulty scaling????
		// Adjust random weight based on base count diff
		// need: total base count
		// 	AI base count
		//  Player Base count
		// 3 situations - even bases (mult is 1?),
		// 								very short bases (mult is closer to 0),
		// 								very ahead bases( mult is > 1)
		// weight neutral less than player?
		var compUnitCount = this.compUnits(bases);
		var playerUnitCount = this.playerUnits(bases);
		var baseScale = compUnitCount / playerUnitCount;
		if (selectedBase.value < 15) return undefined;
		for (var i = 0; i < bases.length; i++) {
			if (shuffledBases[i].team !== 'cpu'
				&& (selectedBase.value * baseScale) > shuffledBases[i].value) {
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

	AI.prototype.compUnits = function (bases) {
		var unitCount = 0;
		for (var i = 0; i < bases.length; i++) {
			if (bases[i].team === 'cpu') {
				unitCount += bases[i].value;
			}
		}
		return unitCount;
	};

	AI.prototype.playerUnits = function (bases) {
		var unitCount = 0;
		for (var i = 0; i < bases.length; i++) {
			if (bases[i].team === 'player') {
				unitCount += bases[i].value;
			}
		}
		return unitCount;
	};

})();
