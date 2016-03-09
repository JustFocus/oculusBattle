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
		console.log(largestBase);
		return largestBase;
	};

	AI.prototype.attackBase = function (bases) {
		var smallestBase = {value: 100};
		for (var i = 0; i < bases.length; i++) {
			if (bases[i].value < smallestBase.value && bases[i].team != 'cpu') {
				smallestBase = bases[i];
			}
		}
		console.log(smallestBase);
		return smallestBase;
	};

})();
