# -*- coding: utf-8 -*-

from objs.cmd import Command
from include.ansi import *

class CmdObj(Command):

    def cmd(self, ob, line):
        ob['머리말'] = ''
        ob.sendLine('☞ 머리말을 제거 하였습니다.')
