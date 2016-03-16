(function() {
	if (typeof(Oculus) === 'undefined') {
		window.Oculus = {};
	}

	var Utils = Oculus.Utils = {};

	var inherits = Utils.inherits = function(child, parent) {
		var Surrogate = function () { this.constructor = child; };
		Surrogate.prototype = parent.prototype;
		child.prototype = new Surrogate();
	};

	var calcVec = Utils.calcVec = function (startPos, endPos, speed) {
		var xDist = (startPos[0] - endPos[0]);
		var yDist = (startPos[1] - endPos[1]);

		var theta = Math.atan(yDist / xDist);
		var xVel = speed * Math.cos(theta);
		var yVel = speed  * Math.sin(theta);
		if (startPos[0] > endPos[0]){ yVel *= -1; xVel *= -1;}
		return [xVel, yVel];
	};

	var dist = Utils.dist = function (pos1, pos2) {
		return Math.sqrt(
			Math.pow(pos1[0] - pos2[0], 2) + Math.pow(pos1[1] - pos2[1], 2)
		);
	};

})();
